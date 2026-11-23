package dev.ruurd.photos.features.viewer

import android.content.Intent
import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.MediaFileSharer
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.database.entity.toTimelineProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoSource
import dev.ruurd.photos.data.repository.TimelineProjectionState
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.model.FullResEligibility
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaFileNames
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.ImmutableMap
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.toPersistentMap
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
    val isInfoSheetOpen: Boolean = false,
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
) : ViewModel() {

    private val _uiState = MutableStateFlow(
        MediaViewerUiState(activeMediaId = initialMediaId),
    )
    val uiState: StateFlow<MediaViewerUiState> = _uiState.asStateFlow()

    private var settleJob: Job? = null
    private val fullResJobs = ConcurrentHashMap<String, Job>()

    init {
        // Frame 1 in 0ms: check if projection store is already loaded
        val currentStore = timelineProjectionStore.state.value
        if (currentStore is TimelineProjectionState.Loaded) {
            val idx = currentStore.items.indexOfFirst { it.id == initialMediaId }
            val initialDetails = currentStore.items.getOrNull(idx)
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
    }

    fun onPageChanged(mediaId: String) {
        _uiState.update { current ->
            if (current.activeMediaId != mediaId) {
                val newIndex = current.items.indexOfFirst { it.id == mediaId }
                val newItem = if (newIndex >= 0) current.items[newIndex] else null
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
    }

    /**
     * Called ~130 ms after the pager settles: loads `/item` and local/offline info for the settled
     * page plus its neighbors so swiping with the info panel open never blanks out metadata.
     */
    fun onSettledPage(mediaId: String) {
        settleJob?.cancel()
        settleJob = viewModelScope.launch {
            val item = _uiState.value.items.firstOrNull { it.id == mediaId } ?: return@launch

            loadInfoForItem(item)
            requestFullRes(item)
            prefetchNeighbors(mediaId)
            trimWindowCaches(mediaId)
        }
    }

    /** Loads a cloud-only original right away, e.g. when the user pinches in before the settle. */
    fun requestFullRes(item: TimelineMediaProjection) {
        val cloudId = item.cloudId ?: return
        if (item.isVideo || _uiState.value.fullResFiles.containsKey(item.id)) return
        if (fullResJobs.containsKey(cloudId)) return

        fullResJobs[cloudId] = viewModelScope.launch(dispatchers.io) {
            try {
                val details = mediaDetailsRepository.get(cloudId) ?: return@launch
                val eligible = FullResEligibility.shouldFetchFullRes(
                    isVideo = details.isVideo,
                    mimeType = details.mimeType,
                    missingSince = details.missingSince,
                )
                if (!eligible) return@launch
                val fileName = details.fileName
                    ?: MediaFileNames.fallbackName(cloudId, isVideo = false, mimeType = details.mimeType)
                val file = fullResFileCache.get(cloudId, fileName) ?: return@launch
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
                fullResJobs.remove(cloudId)
            }
        }
    }

    suspend fun shareIntent(): Intent? {
        val state = _uiState.value
        val item = state.items.firstOrNull { it.id == state.activeMediaId } ?: return null
        return mediaFileSharer.shareIntent(item, state.activeDetails)
    }

    suspend fun copyToClipboard(): Boolean {
        val state = _uiState.value
        val item = state.items.firstOrNull { it.id == state.activeMediaId } ?: return false
        return mediaFileSharer.copyToClipboard(item, state.activeDetails)
    }

    fun openInfoSheet() {
        _uiState.update { it.copy(isInfoSheetOpen = true) }
    }

    fun closeInfoSheet() {
        _uiState.update { it.copy(isInfoSheetOpen = false) }
    }

    fun toggleChrome() {
        _uiState.update { it.copy(isChromeVisible = !it.isChromeVisible) }
    }

    fun setChromeVisible(visible: Boolean) {
        _uiState.update { it.copy(isChromeVisible = visible) }
    }

    private suspend fun loadInfoForItem(item: TimelineMediaProjection) {
        item.cloudId?.let(mediaDetailsRepository::peek)?.let { cached ->
            publishItemInfo(item.id, details = cached, localInfo = null)
        }

        val details = item.cloudId?.let { mediaDetailsRepository.get(it) }
        val localInfo = readLocalInfo(item, hasDetails = details != null)
        publishItemInfo(item.id, details = details, localInfo = localInfo)
    }

    private fun publishItemInfo(
        mediaId: String,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
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
            if (nextDetails === current.detailsById && nextLocalInfo === current.localInfoById) {
                current
            } else {
                current.copy(detailsById = nextDetails, localInfoById = nextLocalInfo)
            }
        }
    }

    private suspend fun readLocalInfo(
        item: TimelineMediaProjection,
        hasDetails: Boolean,
    ): LocalMediaInfo? {
        item.localContentUri?.let { uri ->
            return localMediaInfoReader.read(uri, item.isVideo)
        }
        // Cloud-only and offline: Room still knows the size, duration and capture time.
        if (hasDetails) return null
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
            if (nextFullRes === current.fullResFiles &&
                nextDetails === current.detailsById &&
                nextLocalInfo === current.localInfoById
            ) {
                current
            } else {
                current.copy(
                    fullResFiles = nextFullRes,
                    detailsById = nextDetails,
                    localInfoById = nextLocalInfo,
                )
            }
        }
    }
}