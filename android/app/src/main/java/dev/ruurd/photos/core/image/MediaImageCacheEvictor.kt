package dev.ruurd.photos.core.image

import coil3.ImageLoader
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import kotlinx.coroutines.withContext

/**
 * Drops every cached decode of one item after its bytes or orientation changed, so the viewer and
 * the timeline tiles stop painting the pre-rotation frame.
 */
class MediaImageCacheEvictor(
    private val imageLoader: ImageLoader,
    private val baseUrlProvider: () -> String,
    private val dispatchers: AppDispatchers,
    private val memoryCacheKeyRegistry: MemoryCacheKeyRegistry? = null,
    private val imageRevisions: MediaImageRevisions? = null,
) {
    suspend fun evict(item: TimelineMediaProjection) = withContext(dispatchers.io) {
        val memoryCache = imageLoader.memoryCache
        memoryCacheKeyRegistry?.remove(item.id)?.let { registeredKey ->
            memoryCache?.remove(registeredKey)
        }
        if (memoryCache != null) {
            memoryCache.keys
                .filter { key -> matches(key.key, item.cloudId, item.localContentUri) }
                .forEach(memoryCache::remove)
        }
        val diskCache = imageLoader.diskCache
        if (diskCache != null) {
            val baseUrl = baseUrlProvider()
            MediaImageRequests.previewDiskCacheKeys(
                baseUrl = baseUrl,
                localContentUri = item.localContentUri,
                cloudId = item.cloudId,
                isVideo = item.isVideo,
                hasThumbnails = item.hasThumbnails,
            ).forEach(diskCache::remove)

            val cloudId = item.cloudId
            if (!cloudId.isNullOrBlank() && baseUrl.isNotBlank()) {
                for (heightPx in CalculateTimelineLayoutUseCase.THUMBNAIL_LADDER_PX) {
                    diskCache.remove("$baseUrl/thumbnails/$cloudId/${heightPx}p.avif")
                    diskCache.remove("$baseUrl/api/photos/$cloudId/thumbnail?size=$heightPx")
                }
            }
        }
        imageRevisions?.bump(item.id)
    }

    /** Cloud URLs and full-res file paths contain the cloudId; local previews are keyed by URI. */
    private fun matches(key: String, cloudId: String?, localContentUri: String?): Boolean = (cloudId != null && key.contains(cloudId)) ||
        (localContentUri != null && (key == localContentUri || key.contains("$localContentUri?size=")))
}
