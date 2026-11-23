package dev.ruurd.photos.core.common

import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * Keeps the viewer's caches honest after a timeline sync: `/item` metadata edited on the web client
 * is dropped so the Info sheet refetches, and originals of deleted items leave the disk cache.
 * Upserts do not evict `full_res_cache`, since most of them are caption or album changes that leave
 * the photo bytes untouched.
 */
interface MediaRefreshCoordinator {
    fun onCloudItemsChanged(upserted: Collection<String>, deleted: Collection<String>)
}

class DefaultMediaRefreshCoordinator(
    private val mediaDetailsRepository: MediaDetailsRepository,
    private val fullResFileCache: FullResFileStore,
    private val scope: CoroutineScope,
) : MediaRefreshCoordinator {

    override fun onCloudItemsChanged(upserted: Collection<String>, deleted: Collection<String>) {
        val changed = upserted + deleted
        if (changed.isEmpty()) return
        mediaDetailsRepository.evict(changed)
        deleted.forEach { cloudId ->
            scope.launch { fullResFileCache.evict(cloudId) }
        }
    }
}
