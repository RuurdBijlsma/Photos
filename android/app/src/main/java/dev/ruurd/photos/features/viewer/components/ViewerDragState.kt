package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/**
 * The 2D finger drag that dismisses the viewer: the offset the pager follows, plus the spring that
 * returns it to rest when the release does not dismiss.
 */
@Stable
internal class ViewerDragState(
    private val scope: CoroutineScope,
) {
    private var isActivelyDragging by mutableStateOf(false)
    private var rawOffset by mutableStateOf(Offset.Zero)
    private val settleOffset = Animatable(Offset.Zero, Offset.VectorConverter)
    private var settleJob: Job? = null

    val isDragging: Boolean get() = isActivelyDragging

    val offset: Offset
        get() = if (isActivelyDragging) rawOffset else settleOffset.value

    val distanceY: Float get() = offset.y.coerceAtLeast(0f)

    val rawDragOffset: Offset get() = rawOffset

    /** `1f` at rest, shrinking to `0.55f` as the photo is pulled down. */
    val scale: Float get() = (1f - (distanceY / 1400f)).coerceIn(0.55f, 1f)

    val backdropAlpha: Float get() = (1f - (distanceY / 500f)).coerceIn(0f, 1f)

    val chromeAlpha: Float get() = (1f - (distanceY / 90f)).coerceIn(0f, 1f)

    fun beginDrag(fromTotal: Offset) {
        settleJob?.cancel()
        settleJob = null
        rawOffset = settleOffset.value + fromTotal
        isActivelyDragging = true
    }

    fun dragBy(delta: Offset) {
        rawOffset += delta
    }

    /** Cancels the return spring; used when the viewer starts closing instead. */
    fun cancelSettle() {
        settleJob?.cancel()
        settleJob = null
    }

    fun stopDragging() {
        isActivelyDragging = false
    }

    /** Springs the released offset back to rest, then reports it. */
    fun settleBack(onSettled: () -> Unit) {
        val releaseOffset = rawOffset
        settleJob?.cancel()
        settleJob = scope.launch {
            settleOffset.snapTo(releaseOffset)
            isActivelyDragging = false
            settleOffset.animateTo(
                targetValue = Offset.Zero,
                animationSpec = spring(
                    dampingRatio = Spring.DampingRatioNoBouncy,
                    stiffness = Spring.StiffnessMedium,
                ),
            )
            onSettled()
        }
    }
}
