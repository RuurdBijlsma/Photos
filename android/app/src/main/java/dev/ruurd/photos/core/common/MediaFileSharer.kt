package dev.ruurd.photos.core.common

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.webkit.MimeTypeMap
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.core.image.ShareCache
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.await
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException
import java.util.Locale

interface MediaFileSharer {
    /** `null` when the item is missing on the server and its local bytes are unreachable. */
    suspend fun shareIntent(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo? = null,
    ): Intent?

    /** Copies a photo's URI, staging local and cloud photos alike so paste targets can always read it. */
    suspend fun copyToClipboard(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo? = null,
    ): Boolean
}

/**
 * Share and Copy both hand the receiving app a real file. Local items pass their readable
 * `content://` URI straight through; cloud originals are staged into [ShareCache] behind
 * [androidx.core.content.FileProvider] so they keep their filename and extension and stay readable
 * for targets without `READ_MEDIA_IMAGES`.
 */
class DefaultMediaFileSharer(
    context: Context,
    private val shareCache: ShareCache,
    private val fullResFileCache: FullResFileStore,
    private val okHttpClient: OkHttpClient,
    private val sessionManager: SessionManager,
    private val dispatchers: AppDispatchers,
) : MediaFileSharer {
    private val appContext = context.applicationContext
    private val resolver = appContext.contentResolver

    override suspend fun shareIntent(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
    ): Intent? = withContext(dispatchers.io) {
        shareCache.prune()

        val localUri = item.localContentUri?.let(Uri::parse)
        if (localUri != null && isReadable(localUri)) {
            val mimeType = details?.mimeType
                ?: localInfo?.mimeType
                ?: resolver.getType(localUri)
                ?: defaultMime(item.isVideo)
            return@withContext buildShareIntent(localUri, mimeType)
        }

        if (details?.isMissing == true) return@withContext null
        val cloudId = item.cloudId ?: return@withContext null
        val staged = stageCloudFile(cloudId, folderKey(item), details, item.isVideo)
            ?: return@withContext null
        val shareMime = details?.mimeType
            ?: mimeFromFilename(staged.name, item.isVideo)
        buildShareIntent(shareCache.uriFor(staged), shareMime)
    }

    /** Copies a photo's URI, staging local and cloud photos alike so paste targets can always read it. */
    override suspend fun copyToClipboard(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
    ): Boolean = withContext(dispatchers.io) {
        if (item.isVideo) return@withContext false
        shareCache.prune()

        val localUri = item.localContentUri?.let(Uri::parse)
        val resolvedMime = details?.mimeType
            ?: localInfo?.mimeType
            ?: localUri?.let(resolver::getType)
        val knownName = details?.fileName ?: localInfo?.fileName
        val staged = stagedPhoto(
            item = item,
            localUri = localUri,
            details = details,
            knownFileName = knownName,
            mimeType = resolvedMime,
        ) ?: return@withContext false
        val uri = shareCache.uriFor(staged)

        withContext(dispatchers.main) {
            val clipboard = appContext.getSystemService(ClipboardManager::class.java)
                ?: return@withContext false
            clipboard.setPrimaryClip(ClipData.newUri(resolver, staged.name, uri))
            true
        }
    }

    private suspend fun stagedPhoto(
        item: TimelineMediaProjection,
        localUri: Uri?,
        details: MediaDetails?,
        knownFileName: String?,
        mimeType: String?,
    ): File? {
        val expectedName = knownFileName?.let {
            MediaFileNames.sanitize(it, isVideo = false, mimeType = mimeType)
        }
        shareCache.peek(folderKey(item), expectedFileName = expectedName)?.let { return it }

        if (localUri != null) {
            val fileName = knownFileName ?: MediaFileNames.fallbackName(
                cloudId = item.cloudId ?: item.id,
                isVideo = false,
                mimeType = mimeType,
            )
            val staged = shareCache.stageInto(
                id = folderKey(item),
                fileName = fileName,
                isVideo = false,
                mimeType = mimeType,
            ) { output ->
                resolver.openInputStream(localUri)?.use { input -> input.copyTo(output) }
                    ?: throw IOException("Unreadable local URI")
            }
            if (staged != null) return staged
        }

        if (details?.isMissing == true) return null
        val cloudId = item.cloudId ?: return null
        return stageCloudFile(cloudId, folderKey(item), details, isVideo = false)
    }

    private suspend fun stageCloudFile(
        cloudId: String,
        key: String,
        details: MediaDetails?,
        isVideo: Boolean,
    ): File? {
        val expectedName = details?.fileName?.let {
            MediaFileNames.sanitize(it, isVideo = isVideo, mimeType = details.mimeType)
        }
        shareCache.peek(key, expectedFileName = expectedName)?.let { return it }

        return if (isVideo) {
            downloadToShareCache(cloudId, key, details?.fileName, details?.mimeType)
        } else {
            val cached = fullResFileCache.get(cloudId, details?.fileName) ?: return null
            val targetName = details?.fileName ?: cached.name
            shareCache.stageInto(
                id = key,
                fileName = targetName,
                isVideo = false,
                mimeType = details?.mimeType,
            ) { output ->
                cached.inputStream().use { it.copyTo(output) }
            }
        }
    }

    /** Large cloud videos bypass the 500 MB photo cache and land in `share_cache` directly. */
    private suspend fun downloadToShareCache(
        cloudId: String,
        key: String,
        knownFileName: String?,
        knownMimeType: String?,
    ): File? = withContext(dispatchers.io) {
        val job = currentCoroutineContext().job
        val request = Request.Builder()
            .url("${sessionManager.getServerUrl()}/api/photos/$cloudId/download")
            .build()
        try {
            okHttpClient.newCall(request).await().use { response ->
                if (!response.isSuccessful) return@withContext null
                val responseMime = knownMimeType
                    ?: response.header("Content-Type")?.substringBefore(';')?.trim()
                val fileName = knownFileName?.takeIf { it.isNotBlank() }
                    ?: MediaFileNames.parseContentDispositionFilename(
                        response.header("Content-Disposition"),
                    )
                    ?: MediaFileNames.fallbackName(
                        cloudId = cloudId,
                        isVideo = true,
                        mimeType = responseMime,
                    )
                val buffer = ByteArray(STREAM_BUFFER_BYTES)
                shareCache.stageInto(
                    id = key,
                    fileName = fileName,
                    isVideo = true,
                    mimeType = responseMime,
                ) { output ->
                    response.body.byteStream().use { input ->
                        while (true) {
                            job.ensureActive()
                            val read = input.read(buffer)
                            if (read < 0) break
                            output.write(buffer, 0, read)
                        }
                    }
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: IOException) {
            null
        } catch (_: Exception) {
            null
        }
    }

    private fun isReadable(uri: Uri): Boolean = try {
        resolver.openInputStream(uri)?.use { true } ?: false
    } catch (_: Exception) {
        false
    }

    private fun buildShareIntent(uri: Uri, mimeType: String): Intent = Intent(Intent.ACTION_SEND).apply {
        type = mimeType
        putExtra(Intent.EXTRA_STREAM, uri)
        clipData = ClipData.newRawUri(null, uri)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }

    private fun folderKey(item: TimelineMediaProjection): String = item.cloudId ?: item.id

    private fun mimeFromFilename(name: String, isVideo: Boolean): String {
        val ext = name.substringAfterLast('.', "").lowercase(Locale.US)
        return when (ext) {
            "jpg", "jpeg" -> "image/jpeg"

            "png" -> "image/png"

            "webp" -> "image/webp"

            "gif" -> "image/gif"

            "avif" -> "image/avif"

            "heic" -> "image/heic"

            "heif" -> "image/heif"

            "tiff", "tif" -> "image/tiff"

            "mp4" -> "video/mp4"

            "mov" -> "video/quicktime"

            "webm" -> "video/webm"

            "mkv" -> "video/x-matroska"

            else -> MimeTypeMap.getSingleton()
                .getMimeTypeFromExtension(ext)
                ?.takeIf { it.isNotBlank() }
                ?: defaultMime(isVideo)
        }
    }

    private fun defaultMime(isVideo: Boolean): String = if (isVideo) "video/*" else "image/*"

    private companion object {
        const val STREAM_BUFFER_BYTES = 16 * 1024
    }
}
