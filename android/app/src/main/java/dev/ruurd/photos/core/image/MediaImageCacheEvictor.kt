package dev.ruurd.photos.core.image

import coil3.ImageLoader
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlinx.coroutines.withContext

/**
 * Drops every cached decode of one item after its bytes or orientation changed, so the viewer and
 * the timeline tiles stop painting the pre-rotation frame.
 */
class MediaImageCacheEvictor(
    private val imageLoader: ImageLoader,
    private val baseUrlProvider: () -> String,
    private val dispatchers: AppDispatchers,
) {
    suspend fun evict(item: TimelineMediaProjection) = withContext(dispatchers.io) {
        val memoryCache = imageLoader.memoryCache
        if (memoryCache != null) {
            memoryCache.keys
                .filter { key -> matches(key.key, item.cloudId, item.localContentUri) }
                .forEach(memoryCache::remove)
        }
        val diskCache = imageLoader.diskCache
        if (diskCache != null) {
            MediaImageRequests.previewDiskCacheKeys(
                baseUrl = baseUrlProvider(),
                localContentUri = item.localContentUri,
                cloudId = item.cloudId,
                isVideo = item.isVideo,
                hasThumbnails = item.hasThumbnails,
            ).forEach(diskCache::remove)
        }
    }

    /** Cloud URLs and full-res file paths contain the cloudId; local previews are keyed by URI. */
    private fun matches(key: String, cloudId: String?, localContentUri: String?): Boolean = (cloudId != null && key.contains(cloudId)) ||
        (localContentUri != null && (key == localContentUri || key.contains("$localContentUri?size=")))
}
