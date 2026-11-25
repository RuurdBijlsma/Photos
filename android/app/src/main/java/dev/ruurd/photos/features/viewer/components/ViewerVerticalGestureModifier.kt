package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.unit.Velocity
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlin.math.abs

private enum class VerticalGestureMode {
    NONE,
    DISMISS_VIEWER,
    INFO_PANEL,
}

/**
 * Unified vertical gesture handler on the untransformed viewer root, running on
 * [PointerEventPass.Main] so child detectors (Telephoto quick-zoom / pinch / pan and
 * HorizontalPager) always take precedence before viewer dismiss or info-sheet drag activates.
 */
internal fun Modifier.viewerVerticalGestures(
    transition: MediaViewerTransitionState,
): Modifier = pointerInput(transition) {
    val velocityTracker = VelocityTracker()
    var lastTapUpTimeMs = 0L
    var lastTapUpPosition = Offset.Zero

    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Main)
        if (transition.isClosing || transition.isOpeningHero || transition.isZoomed) {
            return@awaitEachGesture
        }

        val metrics = transition.metrics
        val panel = transition.panel
        val drag = transition.drag
        val panelAlreadyActive = panel.isOpenBeyondThreshold
        val isQuickZoomCandidate = !panelAlreadyActive &&
            lastTapUpTimeMs > 0L &&
            (down.uptimeMillis - lastTapUpTimeMs) in 0L..metrics.doubleTapTimeoutMs &&
            (down.position - lastTapUpPosition).getDistance() <= metrics.doubleTapSlopPx
        if (isQuickZoomCandidate) {
            // Consume the tap memory and yield the entire second press + drag to Telephoto.
            lastTapUpTimeMs = 0L
            return@awaitEachGesture
        }

        if (panelAlreadyActive) {
            panel.stopFlingUnderFinger()
        }

        val pointerId = down.id
        var totalDx = 0f
        var totalDy = 0f
        var maxDisplacementPx = 0f
        var wasMultiTouch = false
        var gestureMode = VerticalGestureMode.NONE
        var gestureItem: TimelineMediaProjection? = null
        val gestureStartTravel = transition.currentActiveItem?.let(transition::pageInfoPanelTravelPx) ?: 0f
        var lastEventTimeMs = down.uptimeMillis
        var lastSampleTimeMs = down.uptimeMillis

        velocityTracker.resetTracking()
        velocityTracker.addPointerInputChange(down)

        try {
            while (true) {
                val event = awaitPointerEvent(PointerEventPass.Main)
                val change = event.changes.firstOrNull { it.id == pointerId } ?: break
                lastEventTimeMs = change.uptimeMillis

                if (event.changes.count { it.pressed } > 1) {
                    wasMultiTouch = true
                    break
                }

                if (transition.isClosing || transition.isOpeningHero || transition.isZoomed) {
                    break
                }

                val displacement = (change.position - down.position).getDistance()
                if (displacement > maxDisplacementPx) {
                    maxDisplacementPx = displacement
                }

                if (!change.pressed) {
                    // Record clean single-taps (note Telephoto consumes the up event on tap).
                    if (gestureMode == VerticalGestureMode.NONE &&
                        !wasMultiTouch &&
                        !panelAlreadyActive &&
                        maxDisplacementPx <= metrics.touchSlopPx &&
                        (change.uptimeMillis - down.uptimeMillis) < metrics.longPressTimeoutMs
                    ) {
                        lastTapUpTimeMs = change.uptimeMillis
                        lastTapUpPosition = change.position
                    }
                    break
                }

                if (gestureMode == VerticalGestureMode.NONE && change.isConsumed) {
                    // A child (Telephoto quick-zoom / pan or HorizontalPager) claimed it.
                    break
                }

                velocityTracker.addPointerInputChange(change)
                if (!change.changedToUpIgnoreConsumed() &&
                    change.position != change.previousPosition
                ) {
                    lastSampleTimeMs = change.uptimeMillis
                }

                val dx = change.position.x - change.previousPosition.x
                val dy = change.position.y - change.previousPosition.y
                totalDx += dx
                totalDy += dy

                when (gestureMode) {
                    VerticalGestureMode.NONE -> {
                        val isVerticalDrag = abs(totalDy) > metrics.touchSlopPx &&
                            abs(totalDy) > abs(totalDx) * VERTICAL_AXIS_BIAS

                        if (isVerticalDrag) {
                            lastTapUpTimeMs = 0L
                            transition.onVerticalDragStart()
                            if (panelAlreadyActive || totalDy < 0f) {
                                val targetItem = transition.currentActiveItem
                                if (targetItem != null) {
                                    gestureMode = VerticalGestureMode.INFO_PANEL
                                    gestureItem = targetItem
                                    panel.beginDrag(
                                        item = targetItem,
                                        panelAlreadyActive = panelAlreadyActive,
                                        dy = dy,
                                        totalDy = totalDy,
                                    )
                                    change.consume()
                                }
                            } else {
                                gestureMode = VerticalGestureMode.DISMISS_VIEWER
                                val draggedId = transition.items
                                    .getOrNull(transition.pagerState.currentPage)?.id
                                    ?: transition.activeMediaId
                                transition.onHiddenTimelineMediaChanged(draggedId)
                                drag.beginDrag(Offset(totalDx, totalDy))
                                change.consume()
                            }
                        } else if (abs(totalDx) > metrics.touchSlopPx && abs(totalDx) >= abs(totalDy)) {
                            lastTapUpTimeMs = 0L
                            // Horizontal swipe belongs to HorizontalPager (open or closed).
                            break
                        }
                    }

                    VerticalGestureMode.DISMISS_VIEWER -> {
                        change.consume()
                        drag.dragBy(Offset(dx, dy))
                    }

                    VerticalGestureMode.INFO_PANEL -> {
                        change.consume()
                        val targetItem = gestureItem
                        if (targetItem != null) {
                            panel.dragBy(targetItem, dy)
                        }
                    }
                }
            }
        } finally {
            val velocityY = if (lastEventTimeMs - lastSampleTimeMs > STALE_RELEASE_MS) {
                0f
            } else {
                velocityTracker
                    .calculateVelocity(Velocity(metrics.maxFlingVelocityPx, metrics.maxFlingVelocityPx))
                    .y
            }

            when (gestureMode) {
                VerticalGestureMode.DISMISS_VIEWER -> if (!transition.isClosing) {
                    val releaseY = drag.rawDragOffset.y.coerceAtLeast(0f)
                    val shouldDismiss = !transition.isZoomed && (
                        releaseY > metrics.dismissThresholdPx ||
                            (velocityY > 1000f && releaseY > metrics.fastFlickMinDistancePx)
                        )

                    if (shouldDismiss) {
                        val releaseScale = (1f - (releaseY / 1400f)).coerceIn(0.55f, 1f)
                        val releaseAlpha = (1f - (releaseY / 500f)).coerceIn(0f, 1f)
                        transition.startClosing(drag.rawDragOffset, releaseScale, releaseAlpha)
                    } else {
                        drag.settleBack {
                            if (!drag.isDragging && !transition.isClosing) {
                                transition.onHiddenTimelineMediaChanged(null)
                            }
                        }
                    }
                }

                VerticalGestureMode.INFO_PANEL -> {
                    val targetItem = gestureItem
                    if (transition.isClosing || targetItem == null) {
                        panel.cancelDrag()
                    } else if (transition.isZoomed) {
                        panel.closeFromDrag(targetItem)
                    } else {
                        panel.settleAfterDrag(targetItem, velocityY, gestureStartTravel)
                    }
                }

                VerticalGestureMode.NONE -> Unit
            }
        }
    }
}

private const val VERTICAL_AXIS_BIAS = 1.05f
private const val STALE_RELEASE_MS = 100L
