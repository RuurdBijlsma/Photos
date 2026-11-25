package dev.ruurd.photos.features.timeline.components

import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import dev.ruurd.photos.domain.model.TimelineLayoutIndex

/**
 * Keeps the viewport pinned to index 0 when newer months are prepended while already at the top;
 * otherwise `LazyColumn` key-anchoring shifts the scroll position down to the previous top header.
 */
@Composable
internal fun TimelineTopPinEffect(
    listState: LazyListState,
    layout: TimelineLayoutIndex,
) {
    val currentLayout by rememberUpdatedState(layout)
    LaunchedEffect(listState) {
        var lastTopRowId: String? = null
        var atTopBeforeUpdate = true
        var pinnedTopRowId: String? = null

        snapshotFlow {
            Triple(
                currentLayout,
                listState.firstVisibleItemIndex,
                listState.firstVisibleItemScrollOffset,
            )
        }.collect { (activeLayout, firstIndex, firstOffset) ->
            val topRowId = activeLayout.rows.firstOrNull()?.id
            val topChanged = topRowId != null && lastTopRowId != null && topRowId != lastTopRowId
            if (topChanged) {
                pinnedTopRowId = if (atTopBeforeUpdate) topRowId else null
            }
            if (listState.isScrollInProgress) {
                pinnedTopRowId = null
            }
            if (pinnedTopRowId != null &&
                pinnedTopRowId == topRowId &&
                !listState.isScrollInProgress &&
                (firstIndex != 0 || firstOffset != 0)
            ) {
                listState.scrollToItem(0)
                pinnedTopRowId = null
                atTopBeforeUpdate = true
            } else {
                atTopBeforeUpdate = firstIndex == 0 && firstOffset == 0
            }
            lastTopRowId = topRowId
        }
    }
}
