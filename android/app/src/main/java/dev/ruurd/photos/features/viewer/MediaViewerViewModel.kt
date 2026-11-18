package dev.ruurd.photos.features.viewer

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.data.repository.TimelineProjectionState
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class MediaViewerUiState(
    val items: ImmutableList<TimelineMediaProjection> = persistentListOf(),
    val activeMediaId: String,
    val initialIndex: Int = 0,
    val isChromeVisible: Boolean = true
)

class MediaViewerViewModel(
    private val initialMediaId: String,
    private val timelineProjectionStore: TimelineProjectionStore,
    private val mediaRepository: MediaRepository,
    private val dispatchers: AppDispatchers
) : ViewModel() {

    private val _uiState = MutableStateFlow(
        MediaViewerUiState(activeMediaId = initialMediaId)
    )
    val uiState: StateFlow<MediaViewerUiState> = _uiState.asStateFlow()

    init {
        // Frame 1 in 0ms: check if projection store is already loaded
        val currentStore = timelineProjectionStore.state.value
        if (currentStore is TimelineProjectionState.Loaded) {
            val idx = currentStore.items.indexOfFirst { it.id == initialMediaId }.coerceAtLeast(0)
            _uiState.value = MediaViewerUiState(
                items = currentStore.items,
                activeMediaId = initialMediaId,
                initialIndex = idx
            )
        } else {
            // Fallback while store is Loading: fetch single item from repository
            viewModelScope.launch(dispatchers.io) {
                val entity = mediaRepository.getById(initialMediaId)
                if (entity != null && _uiState.value.items.isEmpty()) {
                    val fallbackProjection = TimelineMediaProjection(
                        id = entity.id,
                        cloudId = entity.cloudId,
                        localContentUri = entity.localContentUri,
                        yearMonth = entity.yearMonth,
                        ratio = entity.ratio,
                        isVideo = entity.isVideo,
                        durationMs = entity.durationMs,
                        hasThumbnails = entity.hasThumbnails,
                        syncStatus = entity.syncStatus,
                        sortTimestamp = entity.sortTimestamp,
                        timezoneOffsetMinutes = entity.timezoneOffsetMinutes
                    )
                    _uiState.update { current ->
                        if (current.items.isEmpty()) {
                            current.copy(
                                items = persistentListOf(fallbackProjection),
                                initialIndex = 0
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
                        val currentActiveId = current.activeMediaId
                        val newIndex = state.items.indexOfFirst { it.id == currentActiveId }
                        val safeIndex = if (newIndex >= 0) {
                            newIndex
                        } else {
                            state.items.indexOfFirst { it.id == initialMediaId }.coerceAtLeast(0)
                        }
                        current.copy(
                            items = state.items,
                            initialIndex = safeIndex
                        )
                    }
                }
            }
        }
    }

    fun onPageChanged(mediaId: String) {
        _uiState.update { current ->
            if (current.activeMediaId != mediaId) {
                current.copy(activeMediaId = mediaId)
            } else {
                current
            }
        }
    }

    fun toggleChrome() {
        _uiState.update { it.copy(isChromeVisible = !it.isChromeVisible) }
    }

    fun setChromeVisible(visible: Boolean) {
        _uiState.update { it.copy(isChromeVisible = visible) }
    }

    companion object {
        fun factory(
            initialMediaId: String,
            timelineProjectionStore: TimelineProjectionStore,
            mediaRepository: MediaRepository,
            dispatchers: AppDispatchers
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return MediaViewerViewModel(
                    initialMediaId = initialMediaId,
                    timelineProjectionStore = timelineProjectionStore,
                    mediaRepository = mediaRepository,
                    dispatchers = dispatchers
                ) as T
            }
        }
    }
}
