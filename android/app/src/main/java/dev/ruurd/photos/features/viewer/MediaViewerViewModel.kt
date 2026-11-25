package dev.ruurd.photos.features.viewer

import android.content.Intent
import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.MediaFileSharer
import dev.ruurd.photos.core.common.MediaRefreshCoordinator
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.database.entity.toTimelineProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoSource
import dev.ruurd.photos.data.repository.TimelineProjectionState
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.model.FullResEligibility
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.ImmutableMap
import kotlinx.collections.immutable.ImmutableSet
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.persistentSetOf
import kotlinx.collections.immutable.toPersistentMap
import kotlinx.collections.immutable.toPersistentSet
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.io.File
import java.util.concurrent.ConcurrentHashMap

@Immutable
data class MediaViewerUiState(
    val items: ImmutableList<TimelineMediaProjection> = persistentListOf(),
    val activeMediaId: String,
    val initialIndex: Int = 0,
    val isChromeVisible: Boolean = true,
    val isActiveItemMissing: Boolean = false,
    val isProjectionLoading: Boolean = false,
    /** `GET /api/photos/{cloudId}/item` keyed by media id, warmed for active + neighbors. */
    val detailsById: ImmutableMap<String, MediaDetails> = persistentMapOf(),
    /** `MediaStore`/EXIF (or Room, for cloud-only items) fallbacks keyed by media id. */
    val localInfoById: ImmutableMap<String, LocalMediaInfo> = persistentMapOf(),
    /** Downloaded originals for cloud-only photos, keyed by media id, so the pager can upgrade. */
    val fullResFiles: ImmutableMap<String, File> = persistentMapOf(),
    val loadingDetailsIds: ImmutableSet<String> = persistentSetOf(),
    val failedDetailsIds: ImmutableSet<String> = persistentSetOf(),
    val isSharing: Boolean = false,
    val isCopying: Boolean = false,
) {
    val activeDetails: MediaDetails?
        get() = detailsById[activeMediaId]

    val activeLocalInfo: LocalMediaInfo?
        get() = localInfoById[activeMediaId]
}

class MediaViewerViewModel(
    private val initialMediaId: String,
    private val timelineProjectionStore: TimelineProjectionStore,
    private val mediaRepository: MediaRepository,
    private val mediaDetailsRepository: MediaDetailsRepository,
    private val fullResFileCache: FullResFileStore,
    private val localMediaInfoReader: LocalMediaInfoSource,
    private val mediaFileSharer: MediaFileSharer,
    private val dispatchers: AppDispatchers,
    mediaRefreshCoordinator: MediaRefreshCoordinator? = null,
) : ViewModel() {

    private val _uiState = MutableStateFlow(
        MediaViewerUiState(activeMediaId = initialMediaId),
    )
    val uiState: StateFlow<MediaViewerUiState> = _uiState.asStateFlow()

    private var currentItemInfoJob: Job? = null
    private var settleJob: Job? = null
    private val fullResJobs = ConcurrentHashMap<String, Job>()

    init {
        // Frame 1 in 0ms: check if projection store is already loaded
        val currentStore = timelineProjectionStore.state.value
        if (currentStore is TimelineProjectionState.Loaded) {
            val idx = currentStore.items.indexOfFirst { it.id == initialMediaId }
            val initialItem = currentStore.items.getOrNull(idx)
            val initialDetails = initialItem
                ?.cloudId
                ?.let(mediaDetailsRepository::peek)
            _uiState.value = MediaViewerUiState(
                items = currentStore.items,
                activeMediaId = initialMediaId,
                initialIndex = idx.coerceAtLeast(0),
                isActiveItemMissing = currentStore.items.isEmpty() || idx < 0,
                isProjectionLoading = false,
                detailsById = if (initialDetails != null) {
                    persistentMapOf(initialMediaId to initialDetails)
                } else {
                    persistentMapOf()
                },
            )
            if (initialItem != null) {
                loadActiveItemInfo(initialItem)
            }
        } else {
            _uiState.value = MediaViewerUiState(
                activeMediaId = initialMediaId,
                isProjectionLoading = true,
            )
            // Fallback while store is Loading: fetch single item from repository
            viewModelScope.launch(dispatchers.io) {
                val entity = mediaRepository.getById(initialMediaId)
                if (entity != null) {
                    val fallbackProjection = entity.toTimelineProjection()
                    _uiState.update { current ->
                        if (current.items.isEmpty() &&
                            timelineProjectionStore.state.value is TimelineProjectionState.Loading
                        ) {
                            current.copy(
                                items = persistentListOf(fallbackProjection),
                                initialIndex = 0,
                                isActiveItemMissing = false,
                                isProjectionLoading = true,
                            )
                        } else {
                            current
                        }
                    }
                    loadActiveItemInfo(fallbackProjection)
                }
            }
        }

        // Observe continuous Stage 2 emissions from TimelineProjectionStore
        viewModelScope.launch(dispatchers.default) {
            timelineProjectionStore.state.collectLatest { state ->
                if (state is TimelineProjectionState.Loaded) {
                    _uiState.update { current ->
                        if (state.items.isEmpty()) {
                            return@update current.copy(
                                items = state.items,
                                initialIndex = 0,
                                isActiveItemMissing = true,
                                isProjectionLoading = false,
                            )
                        }

                        val activeIdx = state.items.indexOfFirst { it.id == current.activeMediaId }
                        if (activeIdx >= 0) {
                            current.copy(
                                items = state.items,
                                initialIndex = activeIdx,
                                isActiveItemMissing = false,
                                isProjectionLoading = false,
                            )
                        } else {
                            // Active item was deleted from Room while open: clamp to nearest index
                            val clampedIdx = current.initialIndex.coerceIn(0, state.items.lastIndex)
                            current.copy(
                                items = state.items,
                                activeMediaId = state.items[clampedIdx].id,
                                initialIndex = clampedIdx,
                                isActiveItemMissing = false,
                                isProjectionLoading = false,
                            )
                        }
                    }
                }
            }
        }

        if (mediaRefreshCoordinator != null) {
            viewModelScope.launch {
                mediaRefreshCoordinator.changedCloudIds.collect(::invalidateCloudIds)
            }
        }
    }

    fun onPageChanged(mediaId: String) {
        var newActiveItem: TimelineMediaProjection? = null
        var changed = false

        _uiState.update { current ->
            if (current.activeMediaId != mediaId) {
                changed = true
                val newIndex = current.items.indexOfFirst { it.id == mediaId }
                val newItem = if (newIndex >= 0) current.items[newIndex] else null
                newActiveItem = newItem
                val peekedDetails = if (newItem != null && !current.detailsById.containsKey(mediaId)) {
                    newItem.cloudId?.let(mediaDetailsRepository::peek)
                } else {
                    null
                }
                current.copy(
                    activeMediaId = mediaId,
                    initialIndex = if (newIndex >= 0) newIndex else current.initialIndex,
                    detailsById = if (peekedDetails != null) {
                        (current.detailsById + (mediaId to peekedDetails)).toPersistentMap()
                    } else {
                        current.detailsById
                    },
                )
            } else {
                current
            }
        }

        if (changed) {
            val activeCloudId = newActiveItem?.cloudId
            cancelFullResExcept(activeCloudId)
            // Start/join active item fetch before cancelling settleJob so an already-in-flight
            // neighbor prefetch for this item is reused instead of aborted and restarted.
            newActiveItem?.let(::loadActiveItemInfo)
            settleJob?.cancel()
            settleJob = null
        }
    }

    /**
     * Called ~130 ms after the pager settles: requests full-res for cloud-only photos, warms
     * neighbor `/item` + local fallbacks in parallel, and trims out-of-window caches.
     */
    fun onSettledPage(mediaId: String) {
        settleJob?.cancel()
        settleJob = viewModelScope.launch {
            val item = _uiState.value.items.firstOrNull { it.id == mediaId } ?: return@launch

            if (currentItemInfoJob?.isActive != true) {
                loadActiveItemInfo(item)
            }
            requestFullRes(item)
            prefetchNeighbors(mediaId)
            trimWindowCaches(mediaId)
        }
    }

    /** Retries `/item` and full-res download when connectivity returns while viewing a cloud item. */
    fun retryActiveItemIfNeeded() {
        val state = _uiState.value
        val item = state.items.firstOrNull { it.id == state.activeMediaId } ?: return
        val cloudId = item.cloudId ?: return
        val hasFreshDetails = state.detailsById[item.id]?.cloudId == cloudId &&
            item.id !in state.failedDetailsIds
        if (!hasFreshDetails && currentItemInfoJob?.isActive != true) {
            loadActiveItemInfo(item)
        }
        if (item.localContentUri == null && !item.isVideo && !state.fullResFiles.containsKey(item.id)) {
            requestFullRes(item)
        }
    }

    /** Loads a cloud-only (or fallback) original, e.g. after settle or when pinching in early. */
    fun requestFullRes(item: TimelineMediaProjection) {
        val cloudId = item.cloudId ?: return
        if (item.isVideo || _uiState.value.fullResFiles.containsKey(item.id)) return
        if (fullResJobs.containsKey(cloudId)) return

        lateinit var launched: Job
        launched = viewModelScope.launch(dispatchers.io) {
            try {
                val details = mediaDetailsRepository.get(cloudId) ?: return@launch
                val eligible = FullResEligibility.shouldFetchFullRes(
                    isVideo = details.isVideo,
                    mimeType = details.mimeType,
                    missingSince = details.missingSince,
                )
                if (!eligible) return@launch
                val file = fullResFileCache.get(cloudId, details.fileName) ?: return@launch
                _uiState.update { state ->
                    if (state.items.none { it.id == item.id }) {
                        state
                    } else {
                        state.copy(
                            fullResFiles = (state.fullResFiles + (item.id to file)).toPersistentMap(),
                        )
                    }
                }
            } finally {
                fullResJobs.remove(cloudId, launched)
            }
        }
        fullResJobs[cloudId] = launched
    }

    suspend fun shareIntent(): Intent? {
        val state = _uiState.value
        if (state.isSharing) return null
        val item = state.items.firstOrNull { it.id == state.activeMediaId } ?: return null
        _uiState.update { it.copy(isSharing = true) }
        return try {
            mediaFileSharer.shareIntent(item, state.activeDetails, state.activeLocalInfo)
        } finally {
            _uiState.update { it.copy(isSharing = false) }
        }
    }

    suspend fun copyToClipboard(): Boolean {
        val state = _uiState.value
        if (state.isCopying) return false
        val item = state.items.firstOrNull { it.id == state.activeMediaId } ?: return false
        _uiState.update { it.copy(isCopying = true) }
        return try {
            mediaFileSharer.copyToClipboard(item, state.activeDetails, state.activeLocalInfo)
        } finally {
            _uiState.update { it.copy(isCopying = false) }
        }
    }

    fun toggleChrome() {
        _uiState.update { it.copy(isChromeVisible = !it.isChromeVisible) }
    }

    private fun invalidateCloudIds(changedCloudIds: Set<String>) {
        if (changedCloudIds.isEmpty()) return
        var activeToReload: TimelineMediaProjection? = null

        _uiState.update { current ->
            val affectedIds = buildSet {
                for (item in current.items) {
                    if (item.cloudId in changedCloudIds) add(item.id)
                }
                for ((mediaId, details) in current.detailsById) {
                    if (details.cloudId in changedCloudIds) add(mediaId)
                }
            }
            if (affectedIds.isEmpty()) return@update current

            if (current.activeMediaId in affectedIds) {
                activeToReload = current.items.firstOrNull { it.id == current.activeMediaId }
            }
            current.copy(
                detailsById = current.detailsById.filterKeys { it !in affectedIds }.toPersistentMap(),
                loadingDetailsIds = current.loadingDetailsIds.filter { it !in affectedIds }.toPersistentSet(),
                failedDetailsIds = current.failedDetailsIds.filter { it !in affectedIds }.toPersistentSet(),
            )
        }

        activeToReload?.let(::loadActiveItemInfo)
    }

    private fun loadActiveItemInfo(item: TimelineMediaProjection) {
        currentItemInfoJob?.cancel()
        currentItemInfoJob = viewModelScope.launch {
            loadInfoForItem(item)
        }
    }

    private fun cancelFullResExcept(activeCloudId: String?) {
        val iterator = fullResJobs.entries.iterator()
        while (iterator.hasNext()) {
            val entry = iterator.next()
            if (entry.key != activeCloudId) {
                entry.value.cancel()
                iterator.remove()
            }
        }
    }

    private suspend fun loadInfoForItem(item: TimelineMediaProjection) {
        val cloudId = item.cloudId
        val peeked = cloudId?.let(mediaDetailsRepository::peek)
        if (peeked != null) {
            publishItemInfo(
                mediaId = item.id,
                details = peeked,
                localInfo = null,
                isLoading = false,
                hasFailed = false,
            )
        } else if (cloudId != null) {
            publishLoadingState(item.id, isLoading = true)
        }

        val details = cloudId?.let { mediaDetailsRepository.get(it) }
        val localInfo = readLocalInfo(item, hasDetails = details != null)
        publishItemInfo(
            mediaId = item.id,
            details = details,
            localInfo = localInfo,
            isLoading = false,
            hasFailed = cloudId != null && details == null,
        )
    }

    private fun publishLoadingState(mediaId: String, isLoading: Boolean) {
        _uiState.update { current ->
            if (current.items.none { it.id == mediaId }) return@update current
            val nextLoading = if (isLoading) {
                (current.loadingDetailsIds + mediaId).toPersistentSet()
            } else {
                (current.loadingDetailsIds - mediaId).toPersistentSet()
            }
            val nextFailed = if (isLoading) {
                (current.failedDetailsIds - mediaId).toPersistentSet()
            } else {
                current.failedDetailsIds
            }
            current.copy(loadingDetailsIds = nextLoading, failedDetailsIds = nextFailed)
        }
    }

    private fun publishItemInfo(
        mediaId: String,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
        isLoading: Boolean,
        hasFailed: Boolean,
    ) {
        _uiState.update { current ->
            if (current.items.none { it.id == mediaId }) return@update current
            val nextDetails = if (details != null && current.detailsById[mediaId] != details) {
                (current.detailsById + (mediaId to details)).toPersistentMap()
            } else {
                current.detailsById
            }
            val nextLocalInfo = if (localInfo != null && current.localInfoById[mediaId] != localInfo) {
                (current.localInfoById + (mediaId to localInfo)).toPersistentMap()
            } else {
                current.localInfoById
            }
            val nextLoading = if (isLoading) {
                (current.loadingDetailsIds + mediaId).toPersistentSet()
            } else {
                (current.loadingDetailsIds - mediaId).toPersistentSet()
            }
            val nextFailed = if (hasFailed) {
                (current.failedDetailsIds + mediaId).toPersistentSet()
            } else {
                (current.failedDetailsIds - mediaId).toPersistentSet()
            }
            current.copy(
                detailsById = nextDetails,
                localInfoById = nextLocalInfo,
                loadingDetailsIds = nextLoading,
                failedDetailsIds = nextFailed,
            )
        }
    }

    private suspend fun readLocalInfo(
        item: TimelineMediaProjection,
        hasDetails: Boolean,
    ): LocalMediaInfo? {
        if (hasDetails) return null
        item.localContentUri?.let { uri ->
            return localMediaInfoReader.read(uri, item.isVideo)
        }
        // Cloud-only and offline: Room still knows the size, duration and capture time.
        val entity = mediaRepository.getById(item.id) ?: return null
        return LocalMediaInfo(
            sizeBytes = entity.sizeBytes.takeIf { it > 0L },
            durationMs = entity.durationMs?.toLong(),
            dateTakenMs = entity.sortTimestamp.takeIf { it > 0L },
        )
    }

    private suspend fun prefetchNeighbors(mediaId: String) {
        val items = _uiState.value.items
        val index = items.indexOfFirst { it.id == mediaId }
        if (index < 0) return
        coroutineScope {
            listOf(index - 1, index + 1)
                .filter { it in items.indices }
                .forEach { neighborIdx ->
                    val neighbor = items[neighborIdx]
                    launch { loadInfoForItem(neighbor) }
                }
        }
    }

    /** Keeps in-memory info/file maps bounded around the settled page; disk files stay cached. */
    private fun trimWindowCaches(mediaId: String) {
        val items = _uiState.value.items
        val index = items.indexOfFirst { it.id == mediaId }
        if (index < 0) return
        val fullResKeep = ((index - 1)..(index + 1)).mapNotNull { items.getOrNull(it)?.id }.toSet()
        val infoKeep = ((index - 2)..(index + 2)).mapNotNull { items.getOrNull(it)?.id }.toSet()
        _uiState.update { current ->
            val nextFullRes = if (current.fullResFiles.keys.all { it in fullResKeep }) {
                current.fullResFiles
            } else {
                current.fullResFiles.filterKeys { it in fullResKeep }.toPersistentMap()
            }
            val nextDetails = if (current.detailsById.keys.all { it in infoKeep }) {
                current.detailsById
            } else {
                current.detailsById.filterKeys { it in infoKeep }.toPersistentMap()
            }
            val nextLocalInfo = if (current.localInfoById.keys.all { it in infoKeep }) {
                current.localInfoById
            } else {
                current.localInfoById.filterKeys { it in infoKeep }.toPersistentMap()
            }
            val nextLoading = current.loadingDetailsIds.filter { it in infoKeep }.toPersistentSet()
            val nextFailed = current.failedDetailsIds.filter { it in infoKeep }.toPersistentSet()
            current.copy(
                fullResFiles = nextFullRes,
                detailsById = nextDetails,
                localInfoById = nextLocalInfo,
                loadingDetailsIds = nextLoading,
                failedDetailsIds = nextFailed,
            )
        }
    }
}
