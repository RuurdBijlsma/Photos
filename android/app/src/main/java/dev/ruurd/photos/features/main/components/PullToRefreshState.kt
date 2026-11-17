package dev.ruurd.photos.features.main.components

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.exp

@Stable
class PullToRefreshState(
    private val scope: CoroutineScope,
    private val thresholdRawPx: Float,
    private val maxVisualOffsetPx: Float,
    private val performHaptic: (HapticFeedbackType) -> Unit,
    isOnline: Boolean,
    isRefreshing: Boolean,
    onRefresh: () -> Unit,
    onOfflineRefresh: () -> Unit
) {
    internal var currentIsOnline: Boolean = isOnline
    internal var currentIsRefreshing: Boolean = isRefreshing
    internal var currentOnRefresh: () -> Unit = onRefresh
    internal var currentOnOfflineRefresh: () -> Unit = onOfflineRefresh

    private var rawPullPx by mutableFloatStateOf(0f)
    private var settleJob: Job? = null
    private var wasArmed: Boolean = false

    val isPulled: Boolean
        get() = rawPullPx > 0f

    /** Normalized pull distance (`0f..1f+`), where `1f` means the release threshold is armed. */
    val pullProgress: Float
        get() = if (thresholdRawPx > 0f) (rawPullPx / thresholdRawPx).coerceAtLeast(0f) else 0f

    /** Damped vertical translation in pixels for the timeline grid. */
    val contentOffsetPx: Float
        get() {
            val raw = rawPullPx
            if (raw <= 0f || maxVisualOffsetPx <= 0f) return 0f
            val resistance = maxVisualOffsetPx * 1.35f
            return maxVisualOffsetPx * (1f - exp(-raw / resistance))
        }

    val isPulling: Boolean by derivedStateOf { rawPullPx > 1f }
    val isArmed: Boolean by derivedStateOf { thresholdRawPx > 0f && rawPullPx >= thresholdRawPx }

    /** Increments when the user releases past threshold while offline so the pill can shake. */
    var offlineFeedbackCount by mutableIntStateOf(0)
        private set

    fun onPullDelta(deltaY: Float): Float {
        if (deltaY == 0f) return 0f
        if (deltaY < 0f && rawPullPx <= 0f) return 0f

        settleJob?.cancel()
        settleJob = null

        val previous = rawPullPx
        val maxRaw = thresholdRawPx * MAX_OVERPULL_MULTIPLIER
        val next = (previous + deltaY).coerceIn(0f, maxRaw)
        rawPullPx = next

        val nowArmed = thresholdRawPx > 0f && next >= thresholdRawPx
        if (nowArmed && !wasArmed && !currentIsRefreshing) {
            performHaptic(HapticFeedbackType.GestureThresholdActivate)
        }
        wasArmed = nowArmed

        // Consume the entire downward delta so platform overscroll does not double-stretch.
        return if (deltaY > 0f) deltaY else next - previous
    }

    fun onRelease() {
        if (rawPullPx <= 0f) {
            wasArmed = false
            return
        }

        val armed = thresholdRawPx > 0f && rawPullPx >= thresholdRawPx
        wasArmed = false

        if (armed && !currentIsRefreshing) {
            if (currentIsOnline) {
                performHaptic(HapticFeedbackType.Confirm)
                currentOnRefresh()
            } else {
                performHaptic(HapticFeedbackType.Reject)
                offlineFeedbackCount++
                currentOnOfflineRefresh()
            }
        }

        settleJob?.cancel()
        settleJob = scope.launch {
            animate(
                initialValue = rawPullPx,
                targetValue = 0f,
                animationSpec = spring(
                    dampingRatio = Spring.DampingRatioNoBouncy,
                    stiffness = Spring.StiffnessMediumLow
                )
            ) { value, _ ->
                rawPullPx = value
            }
        }
    }

    fun reset() {
        settleJob?.cancel()
        settleJob = null
        rawPullPx = 0f
        wasArmed = false
    }

    private companion object {
        const val MAX_OVERPULL_MULTIPLIER = 1.6f
    }
}

@Composable
fun rememberPullToRefreshState(
    isOnline: Boolean,
    isRefreshing: Boolean,
    onRefresh: () -> Unit,
    onOfflineRefresh: () -> Unit
): PullToRefreshState {
    val scope = rememberCoroutineScope()
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current

    val thresholdRawPx = with(density) { PULL_THRESHOLD_RAW_DP.dp.toPx() }
    val maxVisualOffsetPx = with(density) { MAX_VISUAL_OFFSET_DP.dp.toPx() }

    val state = remember(scope, haptic, thresholdRawPx, maxVisualOffsetPx) {
        PullToRefreshState(
            scope = scope,
            thresholdRawPx = thresholdRawPx,
            maxVisualOffsetPx = maxVisualOffsetPx,
            performHaptic = { type -> haptic.performHapticFeedback(type) },
            isOnline = isOnline,
            isRefreshing = isRefreshing,
            onRefresh = onRefresh,
            onOfflineRefresh = onOfflineRefresh
        )
    }

    state.currentIsOnline = isOnline
    state.currentIsRefreshing = isRefreshing
    state.currentOnRefresh = onRefresh
    state.currentOnOfflineRefresh = onOfflineRefresh

    return state
}

private const val PULL_THRESHOLD_RAW_DP = 140f
private const val MAX_VISUAL_OFFSET_DP = 100f