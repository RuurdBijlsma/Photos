package dev.ruurd.photos.data.datasource.local

import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.database.Cursor
import android.net.Uri
import android.provider.MediaStore
import dev.ruurd.photos.domain.model.LocalMediaFile
import dev.ruurd.photos.domain.repository.LocalMediaStore

/**
 * Reads every non-pending, non-trashed, non-empty image/video from `MediaStore`.
 * Omitted rows only mean "deleted from the device" when the caller holds full permission for that
 * media type; [dev.ruurd.photos.domain.model.ScanGuards] decides that.
 */
class MediaStoreDataSource(private val context: Context) : LocalMediaStore {

    private val contentResolver: ContentResolver = context.contentResolver

    override suspend fun queryImages(): List<LocalMediaFile> =
        query(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, isVideo = false)

    override suspend fun queryVideos(): List<LocalMediaFile> =
        query(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, isVideo = true)

    override suspend fun generation(): Long =
        MediaStore.getGeneration(context, MediaStore.VOLUME_EXTERNAL)

    private fun query(collection: Uri, isVideo: Boolean): List<LocalMediaFile> {
        val projection = BASE_PROJECTION + if (isVideo) arrayOf(MediaStore.Video.Media.DURATION) else emptyArray<String>()

        return contentResolver.query(collection, projection, SELECTION, null, null)?.use { cursor ->
            val files = ArrayList<LocalMediaFile>(cursor.count)
            while (cursor.moveToNext()) {
                files.add(cursor.toLocalMediaFile(collection, isVideo))
            }
            files
        } ?: emptyList()
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
