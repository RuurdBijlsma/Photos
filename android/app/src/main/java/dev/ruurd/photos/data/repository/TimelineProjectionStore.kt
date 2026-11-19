package dev.ruurd.photos.data.repository

import androidx.compose.runtime.Immutable
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@Immutable
sealed interface TimelineProjectionState {
    data object Loading : TimelineProjectionState
    data class Loaded(val items: ImmutableList<TimelineMediaProjection>) : TimelineProjectionState
}

/**
 * Shared Stage 2 timeline projection store across TimelineViewModel and MediaViewerViewModel.
 * Modeled as Loading vs Loaded(items) (never initialized to empty) so MediaViewerViewModel
 * does not mistake an uninitialized store for a deleted library.
 */
class TimelineProjectionStore(
    private val mediaRepository: MediaRepository,
    private val scope: CoroutineScope,
    private val dispatchers: AppDispatchers,
) {
    private val _state = MutableStateFlow<TimelineProjectionState>(TimelineProjectionState.Loading)
    val state: StateFlow<TimelineProjectionState> = _state.asStateFlow()

    private var job: Job? = null

    init {
        start()
    }

    fun start() {
        if (job?.isActive == true) return
        job = scope.launch(dispatchers.io) {
            mediaRepository.getTimelineStage2().collect { list ->
                _state.value = TimelineProjectionState.Loaded(list.toPersistentList())
            }
        }
    }

    fun clear() {
        job?.cancel()
        job = null
        _state.value = TimelineProjectionState.Loading
        start()
    }

    fun stop() {
        job?.cancel()
        job = null
    }

    fun getById(id: String): TimelineMediaProjection? {
        val current = _state.value
        return if (current is TimelineProjectionState.Loaded) {
            current.items.firstOrNull { it.id == id }
        } else {
            null
        }
    }
}
