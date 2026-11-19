package dev.ruurd.photos.core.image

import android.content.Context
import android.net.Uri
import coil3.memory.MemoryCache
import coil3.request.ImageRequest
import coil3.size.Dimension
import coil3.size.Size as CoilSize

/**
 * Single source of truth for preview models, cloud thumbnail URLs and their Coil requests, shared by
 * the timeline grid, the hero transition overlay, the zoomable viewer page and the neighbor
 * prefetcher. Pre-generated AVIF thumbnails are served without auth; items whose thumbnails are not
 * ready use the on-demand endpoint, which needs the session cookie (sent by the shared OkHttp client).
 */
object MediaImageRequests {

    fun previewHeightPx(isVideo: Boolean): Int =
        if (isVideo) VIDEO_PREVIEW_HEIGHT_PX else PHOTO_PREVIEW_HEIGHT_PX

    /**
     * @param bypassPlatformThumbnail skips `ContentResolver.loadThumbnail`, used by viewer stills so
     * `MediaProvider` cached/EXIF thumbnails never mismatch the primary image crop or quality.
     * @return a [MediaStoreThumbnail] for local items, the cloud URL for cloud-only ones, or `null`
     * when neither is available.
     */
    fun previewModel(
        localContentUri: String?,
        cloudId: String?,
        isVideo: Boolean,
        hasThumbnails: Boolean,
        baseUrl: String,
        heightPx: Int = previewHeightPx(isVideo),
        bypassPlatformThumbnail: Boolean = !isVideo
    ): Any? {
        val cloudUrl = cloudThumbnailUrl(baseUrl, cloudId, hasThumbnails, heightPx)
        val localUri = localContentUri?.let(Uri::parse)
        return if (localUri != null) {
            MediaStoreThumbnail(
                uri = localUri,
                cloudId = cloudId,
                cloudFallbackUrl = cloudUrl,
                isVideo = isVideo,
                bypassPlatformThumbnail = bypassPlatformThumbnail
            )
        } else {
            cloudUrl
        }
    }

    /**
     * @param placeholderKey previous key for the same item, so a re-request paints the cached frame.
     * @param onCacheKey receives the key the request resolved to, for [MemoryCacheKeyRegistry].
     */
    fun previewRequest(
        context: Context,
        localContentUri: String?,
        cloudId: String?,
        isVideo: Boolean,
        hasThumbnails: Boolean,
        baseUrl: String,
        heightPx: Int = previewHeightPx(isVideo),
        bypassPlatformThumbnail: Boolean = !isVideo,
        placeholderKey: MemoryCache.Key? = null,
        onCacheKey: ((MemoryCache.Key) -> Unit)? = null
    ): ImageRequest? {
        val model = previewModel(
            localContentUri = localContentUri,
            cloudId = cloudId,
            isVideo = isVideo,
            hasThumbnails = hasThumbnails,
            baseUrl = baseUrl,
            heightPx = heightPx,
            bypassPlatformThumbnail = bypassPlatformThumbnail
        ) ?: return null

        return ImageRequest.Builder(context)
            .data(model)
            .apply {
                // Local previews decode at a square bound; cloud AVIFs keep their aspect at heightPx.
                if (model is MediaStoreThumbnail) {
                    size(heightPx, heightPx)
                } else {
                    size(CoilSize(Dimension.Undefined, Dimension.Pixels(heightPx)))
                }
                placeholderKey?.let(::placeholderMemoryCacheKey)
                if (onCacheKey != null) {
                    listener(onSuccess = { _, result -> result.memoryCacheKey?.let(onCacheKey) })
                }
            }
            .build()
    }

    private fun cloudThumbnailUrl(
        baseUrl: String,
        cloudId: String?,
        hasThumbnails: Boolean,
        heightPx: Int
    ): String? {
        if (cloudId.isNullOrBlank() || baseUrl.isBlank()) return null
        return if (hasThumbnails) {
            "$baseUrl/thumbnails/$cloudId/${heightPx}p.avif"
        } else {
            "$baseUrl/api/photos/$cloudId/thumbnail?size=$heightPx"
        }
    }

    private const val PHOTO_PREVIEW_HEIGHT_PX = 1440
    private const val VIDEO_PREVIEW_HEIGHT_PX = 720
}
