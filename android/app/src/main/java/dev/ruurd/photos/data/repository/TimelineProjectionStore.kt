package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.repository.MediaRepository
import dev.ruurd.photos.domain.repository.TimelineProjectionSource
import dev.ruurd.photos.domain.repository.TimelineProjectionState
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Shared Stage 2 timeline projection store across TimelineViewModel and MediaViewerViewModel.
 */
class TimelineProjectionStore(
    private val mediaRepository: MediaRepository,
    private val scope: CoroutineScope,
    private val dispatchers: AppDispatchers,
) : TimelineProjectionSource {
    private val _state = MutableStateFlow<TimelineProjectionState>(TimelineProjectionState.Loading)
    override val state: StateFlow<TimelineProjectionState> = _state.asStateFlow()

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
