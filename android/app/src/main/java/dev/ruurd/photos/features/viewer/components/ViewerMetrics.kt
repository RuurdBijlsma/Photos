package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.DecayAnimationSpec
import androidx.compose.animation.rememberSplineBasedDecay
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.unit.dp
import dev.ruurd.photos.ui.theme.PhotosRadii

/**
 * Density-resolved gesture and animation constants for the viewer, so the transition state does not
 * carry a dozen px values through its constructor.
 */
@Immutable
internal class ViewerMetrics(
    val touchSlopPx: Float,
    val doubleTapSlopPx: Float,
    val doubleTapTimeoutMs: Long,
    val longPressTimeoutMs: Long,
    val dismissThresholdPx: Float,
    val fastFlickMinDistancePx: Float,
    val infoCommitThresholdPx: Float,
    val infoFlickVelocityPx: Float,
    val minFlingVelocityPx: Float,
    val maxFlingVelocityPx: Float,
    val sheetCornerOverlapPx: Float,
    val flingDecay: DecayAnimationSpec<Float>,
)

@Composable
internal fun rememberViewerMetrics(): ViewerMetrics {
    val density = LocalDensity.current
    val viewConfig = LocalViewConfiguration.current
    val flingDecay = rememberSplineBasedDecay<Float>()
    return remember(density, viewConfig, flingDecay) {
        with(density) {
            ViewerMetrics(
                touchSlopPx = viewConfig.touchSlop,
                doubleTapSlopPx = DOUBLE_TAP_SLOP_DP.dp.toPx(),
                doubleTapTimeoutMs = viewConfig.doubleTapTimeoutMillis,
                longPressTimeoutMs = viewConfig.longPressTimeoutMillis,
                dismissThresholdPx = DISMISS_THRESHOLD_DP.dp.toPx(),
                fastFlickMinDistancePx = FAST_FLICK_MIN_DISTANCE_DP.dp.toPx(),
                infoCommitThresholdPx = INFO_COMMIT_THRESHOLD_DP.dp.toPx(),
                infoFlickVelocityPx = INFO_FLICK_VELOCITY_DP_PER_SEC.dp.toPx(),
                minFlingVelocityPx = MIN_FLING_VELOCITY_DP_PER_SEC.dp.toPx(),
                maxFlingVelocityPx = viewConfig.maximumFlingVelocity,
                sheetCornerOverlapPx = PhotosRadii.Sheet.toPx(),
                flingDecay = flingDecay,
            )
        }
    }
}

private const val DOUBLE_TAP_SLOP_DP = 100f
private const val DISMISS_THRESHOLD_DP = 110f
private const val FAST_FLICK_MIN_DISTANCE_DP = 40f
private const val INFO_COMMIT_THRESHOLD_DP = 44f
private const val INFO_FLICK_VELOCITY_DP_PER_SEC = 100f
private const val MIN_FLING_VELOCITY_DP_PER_SEC = 50f
