package dev.ruurd.photos.core.common

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.core.image.ShareCache
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException

interface MediaFileSharer {
    /** `null` when the item is missing on the server or its bytes are unreachable. */
    suspend fun shareIntent(item: TimelineMediaProjection, details: MediaDetails?): Intent?

    /** Copies a photo's URI, staging local and cloud photos alike so paste targets can always read it. */
    suspend fun copyToClipboard(item: TimelineMediaProjection, details: MediaDetails?): Boolean
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
    ): Intent? = withContext(dispatchers.io) {
        if (details?.isMissing == true) return@withContext null
        shareCache.prune()

        val localUri = item.localContentUri?.let(Uri::parse)
        if (localUri != null && isReadable(localUri)) {
            val mimeType = details?.mimeType ?: resolver.getType(localUri) ?: defaultMime(item.isVideo)
            return@withContext buildShareIntent(localUri, mimeType)
        }

        val cloudId = item.cloudId ?: return@withContext null
        val staged = stageCloudFile(cloudId, folderKey(item), details, item.isVideo)
            ?: return@withContext null
        buildShareIntent(shareCache.uriFor(staged), details?.mimeType ?: defaultMime(item.isVideo))
    }

    /** Copies a photo's URI, staging local and cloud photos alike so paste targets can always read it. */
    override suspend fun copyToClipboard(
        item: TimelineMediaProjection,
        details: MediaDetails?,
    ): Boolean = withContext(dispatchers.io) {
        if (item.isVideo || details?.isMissing == true) return@withContext false
        shareCache.prune()

        val fileName = details?.fileName ?: MediaFileNames.fallbackName(
            cloudId = item.cloudId ?: item.id,
            isVideo = false,
            mimeType = details?.mimeType,
        )
        val staged = stagedPhoto(item, details, fileName) ?: return@withContext false
        val uri = shareCache.uriFor(staged)

        withContext(dispatchers.main) {
            val clipboard = appContext.getSystemService(ClipboardManager::class.java)
                ?: return@withContext false
            clipboard.setPrimaryClip(ClipData.newUri(resolver, fileName, uri))
            true
        }
    }

    private suspend fun stagedPhoto(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        fileName: String,
    ): File? {
        shareCache.peek(folderKey(item))?.let { return it }

        val localUri = item.localContentUri?.let(Uri::parse)
        if (localUri != null) {
            val staged = shareCache.stageInto(
                id = folderKey(item),
                fileName = fileName,
                isVideo = false,
                mimeType = details?.mimeType,
            ) { output ->
                resolver.openInputStream(localUri)?.use { input -> input.copyTo(output) }
                    ?: throw IOException("Unreadable local URI")
            }
            if (staged != null) return staged
        }

        val cloudId = item.cloudId ?: return null
        return stageCloudFile(cloudId, folderKey(item), details, isVideo = false)
    }

    private suspend fun stageCloudFile(
        cloudId: String,
        key: String,
        details: MediaDetails?,
        isVideo: Boolean,
    ): File? {
        shareCache.peek(key)?.let { return it }
        val fileName = details?.fileName
            ?: MediaFileNames.fallbackName(cloudId, isVideo, details?.mimeType)

        return if (isVideo) {
            downloadToShareCache(cloudId, key, fileName, details?.mimeType)
        } else {
            val cached = fullResFileCache.get(cloudId, fileName) ?: return null
            shareCache.stageInto(key, fileName, isVideo = false, mimeType = details?.mimeType) { output ->
                cached.inputStream().use { it.copyTo(output) }
            }
        }
    }

    /** Large cloud videos bypass the 500 MB photo cache and land in `share_cache` directly. */
    private suspend fun downloadToShareCache(
        cloudId: String,
        key: String,
        fileName: String,
        mimeType: String?,
    ): File? = withContext(dispatchers.io) {
        val request = Request.Builder()
            .url("${sessionManager.getServerUrl()}/api/photos/$cloudId/download")
            .build()
        try {
            okHttpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@withContext null
                shareCache.stageInto(key, fileName, isVideo = true, mimeType = mimeType) { output ->
                    response.body.byteStream().use { it.copyTo(output) }
                }
            }
        } catch (_: IOException) {
            null
        } catch (_: Exception) {
            null
        }
    }

    private fun isReadable(uri: Uri): Boolean = try {
        resolver.openInputStream(uri)?.use { it.read() } != null
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

    private fun defaultMime(isVideo: Boolean): String = if (isVideo) "video/*" else "image/*"
}
