package dev.ruurd.photos.data.datasource.local

import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.database.ContentObserver
import android.database.Cursor
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import dev.ruurd.photos.domain.model.LocalMediaFile
import dev.ruurd.photos.domain.repository.LocalMediaStore
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.buffer
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.debounce

/**
 * Reads every non-pending, non-trashed, non-empty image/video from `MediaStore`.
 * Omitted rows only mean "deleted from the device" when the caller holds full permission for that
 * media type; [dev.ruurd.photos.domain.model.ScanGuards] decides that.
 */
class MediaStoreDataSource(private val context: Context) : LocalMediaStore {

    private val contentResolver: ContentResolver = context.contentResolver

    override suspend fun queryImages(modifiedAfterGeneration: Long?): List<LocalMediaFile> =
        query(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, isVideo = false, modifiedAfterGeneration)

    override suspend fun queryVideos(modifiedAfterGeneration: Long?): List<LocalMediaFile> =
        query(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, isVideo = true, modifiedAfterGeneration)

    override suspend fun queryImageUris(): Set<String> =
        queryUris(MediaStore.Images.Media.EXTERNAL_CONTENT_URI)

    override suspend fun queryVideoUris(): Set<String> =
        queryUris(MediaStore.Video.Media.EXTERNAL_CONTENT_URI)

    override suspend fun generation(): Long =
        MediaStore.getGeneration(context, MediaStore.VOLUME_EXTERNAL)

    override suspend fun version(): String =
        MediaStore.getVersion(context, MediaStore.VOLUME_EXTERNAL)

    @OptIn(FlowPreview::class)
    override fun observeChanges(): Flow<Unit> = callbackFlow {
        val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean, uri: Uri?) {
                trySend(Unit)
            }
        }
        contentResolver.registerContentObserver(
            MediaStore.Images.Media.EXTERNAL_CONTENT_URI,
            true,
            observer
        )
        contentResolver.registerContentObserver(
            MediaStore.Video.Media.EXTERNAL_CONTENT_URI,
            true,
            observer
        )
        awaitClose {
            contentResolver.unregisterContentObserver(observer)
        }
    }
        .buffer(Channel.CONFLATED)
        .debounce(CHANGE_DEBOUNCE_MS)

    private fun query(
        collection: Uri,
        isVideo: Boolean,
        modifiedAfterGeneration: Long?
    ): List<LocalMediaFile> {
        val projection =
            BASE_PROJECTION + if (isVideo) arrayOf(MediaStore.Video.Media.DURATION) else emptyArray<String>()
        val selection = if (modifiedAfterGeneration != null) {
            "$SELECTION AND ${MediaStore.MediaColumns.GENERATION_MODIFIED} > ?"
        } else {
            SELECTION
        }
        val selectionArgs = modifiedAfterGeneration?.let { arrayOf(it.toString()) }

        return contentResolver.query(collection, projection, selection, selectionArgs, null)
            ?.use { cursor ->
                val files = ArrayList<LocalMediaFile>(cursor.count)
                while (cursor.moveToNext()) {
                    files.add(cursor.toLocalMediaFile(collection, isVideo))
                }
                files
            } ?: emptyList()
    }

    private fun queryUris(collection: Uri): Set<String> {
        val projection = arrayOf(MediaStore.MediaColumns._ID)
        return contentResolver.query(collection, projection, SELECTION, null, null)?.use { cursor ->
            val idColumn = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns._ID)
            val uris = HashSet<String>(cursor.count.coerceAtLeast(16))
            while (cursor.moveToNext()) {
                uris.add(ContentUris.withAppendedId(collection, cursor.getLong(idColumn)).toString())
            }
            uris
        } ?: emptySet()
    }

    private fun Cursor.toLocalMediaFile(collection: Uri, isVideo: Boolean): LocalMediaFile {
        val id = getLong(getColumnIndexOrThrow(MediaStore.MediaColumns._ID))
        // DATE_ADDED / DATE_MODIFIED are seconds, DATE_TAKEN is milliseconds.
        val dateTaken = getLongOrNull(MediaStore.MediaColumns.DATE_TAKEN)?.takeIf { it > 0L }
            ?: (getLongOrNull(MediaStore.MediaColumns.DATE_MODIFIED) ?: 0L) * 1000L

        return LocalMediaFile(
            mediaStoreId = id,
            contentUri = ContentUris.withAppendedId(collection, id).toString(),
            bucketId = getStringOrNull(MediaStore.MediaColumns.BUCKET_ID) ?: "",
            bucketName = getStringOrNull(MediaStore.MediaColumns.BUCKET_DISPLAY_NAME),
            relativePath = getStringOrNull(MediaStore.MediaColumns.RELATIVE_PATH),
            sizeBytes = getLongOrNull(MediaStore.MediaColumns.SIZE) ?: 0L,
            width = getIntOrNull(MediaStore.MediaColumns.WIDTH) ?: 0,
            height = getIntOrNull(MediaStore.MediaColumns.HEIGHT) ?: 0,
            orientation = getIntOrNull(COLUMN_ORIENTATION) ?: 0,
            dateTakenMs = dateTaken,
            mimeType = getStringOrNull(MediaStore.MediaColumns.MIME_TYPE),
            isVideo = isVideo,
            durationMs = if (isVideo) getIntOrNull(MediaStore.Video.Media.DURATION) else null
        )
    }

    private fun Cursor.getLongOrNull(column: String): Long? =
        getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let { getLong(it) }

    private fun Cursor.getIntOrNull(column: String): Int? =
        getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let { getInt(it) }

    private fun Cursor.getStringOrNull(column: String): String? =
        getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let { getString(it) }

    private companion object {
        // Column ids only: `MediaStore.MediaColumns.ORIENTATION` is deprecated and may be removed.
        const val COLUMN_ORIENTATION = "orientation"
        const val CHANGE_DEBOUNCE_MS = 300L

        val BASE_PROJECTION = arrayOf(
            MediaStore.MediaColumns._ID,
            MediaStore.MediaColumns.WIDTH,
            MediaStore.MediaColumns.HEIGHT,
            COLUMN_ORIENTATION,
            MediaStore.MediaColumns.SIZE,
            MediaStore.MediaColumns.DATE_TAKEN,
            MediaStore.MediaColumns.DATE_MODIFIED,
            MediaStore.MediaColumns.BUCKET_ID,
            MediaStore.MediaColumns.BUCKET_DISPLAY_NAME,
            MediaStore.MediaColumns.RELATIVE_PATH,
            MediaStore.MediaColumns.MIME_TYPE
        )

        val SELECTION = "${MediaStore.MediaColumns.IS_PENDING} = 0" +
                " AND ${MediaStore.MediaColumns.IS_TRASHED} = 0" +
                " AND ${MediaStore.MediaColumns.SIZE} > 0"
    }
}