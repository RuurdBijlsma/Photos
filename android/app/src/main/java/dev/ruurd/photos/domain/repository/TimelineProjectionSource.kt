package dev.ruurd.photos.domain.repository

import androidx.compose.runtime.Immutable
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlinx.collections.immutable.ImmutableList
import kotlinx.coroutines.flow.StateFlow

@Immutable
sealed interface TimelineProjectionState {
    data object Loading : TimelineProjectionState
    data class Loaded(val items: ImmutableList<TimelineMediaProjection>) : TimelineProjectionState
}

/**
 * Shared Stage 2 timeline stream observed by both the timeline grid and the media viewer.
 * Starts in [TimelineProjectionState.Loading] so consumers never mistake an uninitialized store for
 * an empty library.
 */
interface TimelineProjectionSource {
    val state: StateFlow<TimelineProjectionState>
}
