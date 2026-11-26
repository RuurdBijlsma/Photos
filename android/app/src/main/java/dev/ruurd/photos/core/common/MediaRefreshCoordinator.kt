package dev.ruurd.photos.core.common

import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.launch

/**
 * Keeps the viewer's caches honest after a timeline sync: `/item` metadata edited on the web client
 * is dropped so the Info sheet refetches, and originals of deleted items leave the disk cache.
 * Upserts do not evict `full_res_cache`, since most of them are caption or album changes that leave
 * the photo bytes untouched.
 */
interface MediaRefreshCoordinator {
    val changedCloudIds: Flow<Set<String>>
        get() = emptyFlow()

    fun onCloudItemsChanged(upserted: Collection<String>, deleted: Collection<String>)

    /** Drops cached `/item` metadata for [cloudIds] and lets open screens refetch those rows. */
    fun invalidateCloudItems(cloudIds: Collection<String>) {
        onCloudItemsChanged(upserted = cloudIds, deleted = emptyList())
    }

    /** Runs a timeline delta sync now, e.g. after a reprocess started server-side work. */
    fun syncNow() {}
}

class DefaultMediaRefreshCoordinator(
    private val mediaDetailsRepository: MediaDetailsRepository,
    private val fullResFileCache: FullResFileStore,
    private val scope: CoroutineScope,
    private val syncRunner: suspend () -> Unit = {},
) : MediaRefreshCoordinator {

    private val _changedCloudIds = MutableSharedFlow<Set<String>>(
        extraBufferCapacity = 32,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    override val changedCloudIds: Flow<Set<String>> = _changedCloudIds.asSharedFlow()

    override fun onCloudItemsChanged(upserted: Collection<String>, deleted: Collection<String>) {
        val changed = (upserted + deleted).toSet()
        if (changed.isEmpty()) return
        mediaDetailsRepository.evict(changed)
        deleted.forEach { cloudId ->
            scope.launch { fullResFileCache.evict(cloudId) }
        }
        _changedCloudIds.tryEmit(changed)
    }

    override fun syncNow() {
        scope.launch { syncRunner() }
    }
}
