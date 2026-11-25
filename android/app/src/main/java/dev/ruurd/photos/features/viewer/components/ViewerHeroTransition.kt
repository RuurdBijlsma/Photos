package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.lerp
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.core.image.lerp

@Immutable
data class ClosingHeroState(
    val item: TimelineMediaProjection,
    val startRect: Rect,
    val targetRect: Rect?,
    val startCorners: TileCornerRadii,
    val targetCorners: TileCornerRadii,
    val startBackdropAlpha: Float,
)

/**
 * The tile↔fullscreen hero animation: progress for the opening flight, the fade used when the
 * viewer opens without tile bounds, and the closing flight back to the origin tile.
 */
@Stable
internal class ViewerHeroTransition(initiallyHasOpenSource: Boolean) {

    var hasFinishedOpening by mutableStateOf(!initiallyHasOpenSource)
        private set

    var openingHeroImageReady by mutableStateOf(false)
        private set

    var closingState by mutableStateOf<ClosingHeroState?>(null)
        private set

    private val openProgress = Animatable(if (initiallyHasOpenSource) 0f else 1f)
    private val fallbackFadeIn = Animatable(if (initiallyHasOpenSource) 1f else 0f)
    private val closeProgress = Animatable(0f)

    val openFraction: Float get() = openProgress.value

    val fallbackFadeInFraction: Float get() = fallbackFadeIn.value

    val closingProgress: Float get() = closeProgress.value

    fun onImageReady() {
        openingHeroImageReady = true
    }

    /** Drives the opening hero animation, or the fade-in when opened without source bounds. */
    suspend fun playOpening(
        hasValidOpenSource: Boolean,
        isClosing: () -> Boolean,
        onOpened: () -> Unit,
    ) {
        if (hasValidOpenSource) {
            openProgress.animateTo(
                targetValue = 1f,
                animationSpec = spring(
                    dampingRatio = 0.85f,
                    stiffness = Spring.StiffnessMediumLow,
                ),
            )
            if (!isClosing()) {
                hasFinishedOpening = true
                onOpened()
            }
        } else {
            fallbackFadeIn.animateTo(1f, tween(durationMillis = FALLBACK_FADE_MS))
        }
    }

    /** Drives the closing hero animation; the caller dismisses the viewer when it returns. */
    suspend fun playClosing() {
        val closing = closingState ?: return
        closeProgress.snapTo(0f)
        if (closing.targetRect != null) {
            closeProgress.animateTo(
                targetValue = 1f,
                animationSpec = spring(
                    dampingRatio = 0.88f,
                    stiffness = Spring.StiffnessMediumLow,
                ),
            )
        } else {
            closeProgress.animateTo(1f, tween(durationMillis = FALLBACK_FADE_MS))
        }
    }

    fun startClosing(state: ClosingHeroState) {
        closingState = state
    }

    fun openingRect(sourceRect: Rect, targetRect: Rect): Rect = lerp(sourceRect, targetRect, openProgress.value)

    fun openingCornerRadii(from: TileCornerRadii): TileCornerRadii = lerp(from, TileCornerRadii.Zero, openProgress.value)

    fun closingRect(closing: ClosingHeroState): Rect {
        val targetRect = closing.targetRect
        if (targetRect != null) {
            return lerp(closing.startRect, targetRect, closeProgress.value)
        }
        val shrinkScale = 1f - 0.15f * closeProgress.value
        val cx = closing.startRect.center.x
        val cy = closing.startRect.center.y
        val w = closing.startRect.width * shrinkScale
        val h = closing.startRect.height * shrinkScale
        return Rect(cx - w / 2f, cy - h / 2f, cx + w / 2f, cy + h / 2f)
    }

    fun closingCornerRadii(closing: ClosingHeroState): TileCornerRadii = if (closing.targetRect != null) {
        lerp(closing.startCorners, closing.targetCorners, closeProgress.value)
    } else {
        closing.startCorners
    }

    fun closingAlpha(closing: ClosingHeroState): Float = if (closing.targetRect != null) 1f else (1f - closeProgress.value).coerceIn(0f, 1f)
}

internal const val FALLBACK_FADE_MS = 180
