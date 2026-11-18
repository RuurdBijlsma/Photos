package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.data.repository.TimelineProjectionState
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.map

/**
 * Two-stage timeline stream over the `(is_visible_in_timeline, year_month, sort_timestamp, id)` index:
 * the newest two months first (fast cold-start paint), then the full library. Later Room
 * invalidations are coalesced, so a sync or MediaStore scan re-lays out once per batch instead of
 * once per row.
 */
@OptIn(FlowPreview::class)
class ObserveTimelineUseCase(
    private val mediaRepository: MediaRepository,
    private val dispatchers: AppDispatchers,
    private val timelineProjectionStore: TimelineProjectionStore? = null
) {
    operator fun invoke(): Flow<List<TimelineMediaProjection>> = channelFlow {
        send(mediaRepository.getTimelineStage1().first())
        val stage2Flow: Flow<List<TimelineMediaProjection>> = if (timelineProjectionStore != null) {
            timelineProjectionStore.state
                .filterIsInstance<TimelineProjectionState.Loaded>()
                .map { it.items }
        } else {
            mediaRepository.getTimelineStage2()
        }
        stage2Flow
            .debounce(BACKGROUND_DEBOUNCE_MS)
            .collect { send(it) }
    }.flowOn(dispatchers.io)

    private companion object {
        const val BACKGROUND_DEBOUNCE_MS = 150L
    }
}