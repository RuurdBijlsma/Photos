package dev.ruurd.photos.features.main.components

import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow

/** Scroll-driven visibility of the floating top bar. */
@Stable
class TopBarVisibilityState {
    var hidden: Boolean by mutableStateOf(false)
        internal set

    fun reveal() {
        hidden = false
    }
}

/**
 * Hides the bar on a scroll down and reveals it on a scroll up or near the top. Non-scroll layout
 * shifts (rows prepended above) are ignored, so new months arriving do not toggle the bar.
 */
@Composable
fun rememberTopBarVisibilityState(listState: LazyListState): TopBarVisibilityState {
    val state = remember(listState) { TopBarVisibilityState() }
    LaunchedEffect(listState) {
        var lastIndex = listState.firstVisibleItemIndex
        var lastOffset = listState.firstVisibleItemScrollOffset
        var travelled = 0f
        snapshotFlow {
            Triple(
                listState.isScrollInProgress,
                listState.firstVisibleItemIndex,
                listState.firstVisibleItemScrollOffset,
            )
        }.collect { (isScrolling, index, offset) ->
            if (index == 0 && offset <= HIDE_THRESHOLD_PX.toInt()) {
                state.hidden = false
                if (offset == 0) {
                    travelled = 0f
                    lastIndex = 0
                    lastOffset = 0
                    return@collect
                }
            }
            if (!isScrolling) {
                lastIndex = index
                lastOffset = offset
                travelled = 0f
                return@collect
            }
            // An index change also resets the offset, so weight it far above any real offset delta.
            val delta = (index - lastIndex) * INDEX_WEIGHT_PX + (offset - lastOffset)
            lastIndex = index
            lastOffset = offset
            if (delta == 0f) return@collect
            if (travelled * delta < 0f) travelled = 0f
            travelled += delta
            if (travelled > HIDE_THRESHOLD_PX) state.hidden = true
            if (travelled < -HIDE_THRESHOLD_PX) state.hidden = false
        }
    }
    return state
}

private const val HIDE_THRESHOLD_PX = 96f
private const val INDEX_WEIGHT_PX = 100_000f
