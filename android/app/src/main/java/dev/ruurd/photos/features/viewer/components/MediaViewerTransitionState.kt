package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.lerp
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.Velocity
import coil3.ImageLoader
import coil3.SingletonImageLoader
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.LocalTimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.lerp
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.coroutines.CoroutineScope
import kotlin.math.abs

private enum class VerticalGestureMode {
    NONE,
    DISMISS_VIEWER,
    INFO_PANEL,
}

/**
 * Owns the viewer's hero open/close animation, the drag-to-dismiss gesture, and the inline
 * swipe-up Media Info panel anchored beneath each page in the horizontal pager.
 *
 * The animation state lives in [ViewerHeroTransition], [ViewerInfoPanelState] and [ViewerDragState];
 * this class wires them to the current pager/items and turns their state into the alphas and offsets
 * the screen renders.
 */
@Stable
internal class MediaViewerTransitionState(
    scope: CoroutineScope,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    private val boundsRegistry: TimelineTileBoundsRegistry,
    private val metrics: ViewerMetrics,
    initialPagerState: PagerState,
    initialSize: IntSize,
    initiallyHasOpenSource: Boolean,
    private val systemBars: ViewerSystemBars,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (String) -> Unit,
    onDismissFinished: () -> Unit,
) {
    private val geometry = ViewerGeometry(
        imageLoader = imageLoader,
        memoryCacheKeyRegistry = memoryCacheKeyRegistry,
        sheetCornerOverlapPx = metrics.sheetCornerOverlapPx,
    )
    private val hero = ViewerHeroTransition(initiallyHasOpenSource)
    private val panel = ViewerInfoPanelState(scope, geometry, metrics)
    private val drag = ViewerDragState(scope)

    init {
        geometry.viewerSize = initialSize
    }

    /** Assigned by [rememberMediaViewerTransitionState] on every composition. */
    internal var pagerState: PagerState = initialPagerState
    internal var items: ImmutableList<TimelineMediaProjection> = persistentListOf()
    internal var activeMediaId: String = ""
    internal var isZoomed: Boolean = false
    internal var openSourceRect: Rect? = null
    internal var openSourceCorners: TileCornerRadii = TileCornerRadii.Zero
    internal var onHiddenTimelineMediaChanged: (String?) -> Unit = onHiddenTimelineMediaChanged
    internal var onPreSyncScroll: (String) -> Unit = onPreSyncScroll
    internal var onDismissFinished: (Unit) -> Unit = { onDismissFinished() }

    val viewerSize: IntSize get() = geometry.viewerSize

    // Opening hero animation state
    val hasFinishedOpening: Boolean get() = hero.hasFinishedOpening
    val openingHeroImageReady: Boolean get() = hero.openingHeroImageReady

    // Closing hero animation state
    val closingState: ClosingHeroState? get() = hero.closingState

    val isDraggingInfo: Boolean get() = panel.isDragging

    /** A programmatic open that arrived while zoomed or mid hero-open, retried once those clear. */
    val isInfoPanelRequested: Boolean get() = panel.isRequested

    val isInfoPanelActive: Boolean get() = panel.isActive

    /** True from the instant swipe-up starts until the photo settles back to its resting y-pos. */
    val shouldShowInfoPanel: Boolean by derivedStateOf {
        panel.isOpenOrAnimating && !isClosing && !isOpeningHero
    }

    private val currentActiveItem: TimelineMediaProjection?
        get() = items.getOrNull(pagerState.currentPage)?.takeIf { it.id == activeMediaId }
            ?: items.firstOrNull { it.id == activeMediaId }
            ?: items.getOrNull(pagerState.currentPage)

    val sheetRestTopPx: Float get() = geometry.sheetRestTopPx

    fun pagePhotoTravelPx(item: TimelineMediaProjection): Float = panel.pagePhotoTravelPx(item)

    fun pageInfoPanelTravelPx(item: TimelineMediaProjection): Float = panel.pageTravelPx(item)

    fun pageInfoPanelTopPx(item: TimelineMediaProjection): Float = panel.pageTopPx(item)

    val isInfoCoveringStatusBar: Boolean by derivedStateOf {
        val activeTop = currentActiveItem?.let(::pageInfoPanelTopPx) ?: sheetRestTopPx
        shouldShowInfoPanel && activeTop <= geometry.statusBarHeightPx * 0.5f
    }

    val hasValidOpenSource: Boolean
        get() = openSourceRect != null && viewerSize.width > 0 && viewerSize.height > 0

    val isOpeningHero: Boolean
        get() = hasValidOpenSource && !hero.hasFinishedOpening && hero.closingState == null

    val isClosing: Boolean get() = hero.closingState != null

    val isDragging: Boolean get() = drag.isDragging

    val dragOffset: Offset get() = drag.offset

    val viewerScale: Float
        get() = if (panel.isOpenAnyAmount) 1f else drag.scale

    /** The pager stays invisible until the opening hero has landed (or fades in without a source tile). */
    val pagerAlpha: Float
        get() = when {
            isOpeningHero -> 0f
            hasValidOpenSource -> 1f
            else -> hero.fallbackFadeInFraction
        }

    fun openingRect(sourceRect: Rect, item: TimelineMediaProjection): Rect = hero.openingRect(sourceRect, geometry.fittedRect(item))

    fun openingCornerRadii(): TileCornerRadii = hero.openingCornerRadii(openSourceCorners)

    fun closingRect(closing: ClosingHeroState): Rect = hero.closingRect(closing)

    fun closingCornerRadii(closing: ClosingHeroState): TileCornerRadii = hero.closingCornerRadii(closing)

    fun closingAlpha(closing: ClosingHeroState): Float = hero.closingAlpha(closing)

    val backdropAlpha: Float
        get() {
            val closing = hero.closingState
            return when {
                closing != null ->
                    (closing.startBackdropAlpha * (1f - hero.closingProgress)).coerceIn(0f, 1f)

                isOpeningHero -> hero.openFraction.coerceIn(0f, 1f)

                panel.isOpenAnyAmount -> 1f

                hasValidOpenSource -> drag.backdropAlpha

                else -> (hero.fallbackFadeInFraction * drag.backdropAlpha).coerceIn(0f, 1f)
            }
        }

    fun chromeAlpha(visibilityAlpha: Float): Float {
        val base = when {
            hero.closingState != null -> 0f
            isOpeningHero -> ((hero.openFraction * 1.4f) - 0.2f).coerceIn(0f, 1f) * visibilityAlpha
            hasValidOpenSource -> drag.chromeAlpha * visibilityAlpha
            else -> (hero.fallbackFadeInFraction * drag.chromeAlpha) * visibilityAlpha
        }
        val infoFade = (1f - panel.openProgress * 2.2f).coerceIn(0f, 1f)
        return base * infoFade
    }

    fun backButtonAlpha(visibilityAlpha: Float): Float {
        val normalChrome = when {
            hero.closingState != null -> 0f
            isOpeningHero -> ((hero.openFraction * 1.4f) - 0.2f).coerceIn(0f, 1f) * visibilityAlpha
            hasValidOpenSource -> drag.chromeAlpha * visibilityAlpha
            else -> (hero.fallbackFadeInFraction * drag.chromeAlpha) * visibilityAlpha
        }
        if (hero.closingState != null || isOpeningHero) return normalChrome
        return maxOf(normalChrome, panel.openProgress)
    }

    fun onSizeChanged(size: IntSize) {
        geometry.viewerSize = size
    }

    internal fun setStatusBarHeight(heightPx: Float) {
        geometry.statusBarHeightPx = heightPx
    }

    fun onInfoContentHeightChanged(mediaId: String, heightPx: Int) {
        if (heightPx <= 0 || geometry.infoContentHeight(mediaId) == heightPx) return
        geometry.setInfoContentHeight(mediaId, heightPx)
        if (isClosing) return
        panel.clampScroll(mediaId, items.firstOrNull { it.id == mediaId })
    }

    fun onHorizontalPageSettled(mediaId: String) = panel.onHorizontalPageSettled(mediaId)

    fun onHeroImageReady() = hero.onImageReady()

    /** Drives the opening hero animation, or the fade-in when opened without source bounds. */
    suspend fun playOpening() {
        hero.playOpening(
            hasValidOpenSource = hasValidOpenSource,
            isClosing = { isClosing },
            onOpened = { onHiddenTimelineMediaChanged(null) },
        )
    }

    /** Drives the closing hero animation, then reports the viewer as dismissed. */
    suspend fun playClosing() {
        hero.playClosing()
        if (hero.closingState == null) return
        onHiddenTimelineMediaChanged(null)
        onDismissFinished(Unit)
    }

    fun requestOpenInfoPanel() {
        if (isClosing || panel.isDragging) return
        // Zoom and the opening hero own the gesture surface; retry once they release it.
        if (isZoomed || isOpeningHero) {
            panel.deferRequest()
            return
        }
        panel.requestOpen(currentActiveItem)
    }

    /**
     * Runs a programmatic open that [requestOpenInfoPanel] deferred because the page was zoomed or
     * the hero was still flying in.
     */
    fun retryPendingInfoRequest() {
        if (!panel.isRequested || isZoomed || isOpeningHero) return
        requestOpenInfoPanel()
    }

    fun requestCloseInfoPanel() {
        panel.requestClose(currentActiveItem)
    }

    fun resetInfoPanelImmediate() = panel.resetImmediate()

    fun startClosing(
        releaseOffset: Offset,
        releaseScale: Float,
        releaseBackdropAlpha: Float,
    ) {
        if (isClosing) return
        drag.cancelSettle()
        panel.resetImmediate()

        val activeItem = currentActiveItem

        if (activeItem == null || viewerSize.width <= 0 || viewerSize.height <= 0) {
            systemBars.restoreForTimeline()
            onHiddenTimelineMediaChanged(null)
            onDismissFinished(Unit)
            return
        }

        val w = viewerSize.width.toFloat()
        val h = viewerSize.height.toFloat()
        val fittedRect = geometry.fittedRect(activeItem)

        val startRect: Rect
        val startCorners: TileCornerRadii
        val startAlpha: Float
        val sourceRect = openSourceRect
        if (isOpeningHero && sourceRect != null) {
            val t = hero.openFraction
            startRect = lerp(sourceRect, fittedRect, t)
            startCorners = lerp(openSourceCorners, TileCornerRadii.Zero, t)
            startAlpha = t.coerceIn(0f, 1f)
        } else {
            val centerX = w / 2f + releaseOffset.x
            val centerY = h / 2f + releaseOffset.y
            val startW = fittedRect.width * releaseScale
            val startH = fittedRect.height * releaseScale
            startRect = Rect(
                left = centerX - startW / 2f,
                top = centerY - startH / 2f,
                right = centerX + startW / 2f,
                bottom = centerY + startH / 2f,
            )
            startCorners = TileCornerRadii.Zero
            startAlpha = releaseBackdropAlpha
        }

        val targetRect = boundsRegistry.boundsInRoot(activeItem.id)
        val targetCorners = if (targetRect != null) {
            boundsRegistry.cornerRadii(activeItem.id)
        } else {
            TileCornerRadii.Zero
        }
        if (targetRect == null) {
            onPreSyncScroll(activeItem.id)
        }
        onHiddenTimelineMediaChanged(targetRect?.let { activeItem.id })

        systemBars.restoreForTimeline()
        drag.stopDragging()
        hero.startClosing(
            ClosingHeroState(
                item = activeItem,
                startRect = startRect,
                targetRect = targetRect,
                startCorners = startCorners,
                targetCorners = targetCorners,
                startBackdropAlpha = startAlpha,
            ),
        )
    }

    /**
     * Unified vertical gesture handler on the untransformed viewer root, running on
     * [PointerEventPass.Main] so child detectors (Telephoto quick-zoom / pinch / pan and
     * HorizontalPager) always take precedence before viewer dismiss or info-sheet drag activates.
     */
    fun Modifier.dismissDragModifier(): Modifier = pointerInput(Unit) {
        val velocityTracker = VelocityTracker()

        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Main)
            if (isClosing || isOpeningHero || isZoomed) {
                return@awaitEachGesture
            }

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
            val gestureStartTravel = currentActiveItem?.let(::pageInfoPanelTravelPx) ?: 0f
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

                    if (isClosing || isOpeningHero || isZoomed) {
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
                                if (panelAlreadyActive || totalDy < 0f) {
                                    val targetItem = currentActiveItem
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
                                    val draggedId = items.getOrNull(pagerState.currentPage)?.id
                                        ?: activeMediaId
                                    onHiddenTimelineMediaChanged(draggedId)
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
                    VerticalGestureMode.DISMISS_VIEWER -> if (!isClosing) {
                        val releaseY = drag.rawDragOffset.y.coerceAtLeast(0f)
                        val shouldDismiss = !isZoomed && (
                            releaseY > metrics.dismissThresholdPx ||
                                (velocityY > 1000f && releaseY > metrics.fastFlickMinDistancePx)
                            )

                        if (shouldDismiss) {
                            val releaseScale = (1f - (releaseY / 1400f)).coerceIn(0.55f, 1f)
                            val releaseAlpha = (1f - (releaseY / 500f)).coerceIn(0f, 1f)
                            startClosing(drag.rawDragOffset, releaseScale, releaseAlpha)
                        } else {
                            drag.settleBack {
                                if (!drag.isDragging && !isClosing) {
                                    onHiddenTimelineMediaChanged(null)
                                }
                            }
                        }
                    }

                    VerticalGestureMode.INFO_PANEL -> {
                        val targetItem = gestureItem
                        if (isClosing || targetItem == null) {
                            panel.cancelDrag()
                        } else if (isZoomed) {
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

    // Tracks the previous clean single tap so double-tap + drag yields to Telephoto quick-zoom.
    private var lastTapUpTimeMs: Long = 0L
    private var lastTapUpPosition: Offset = Offset.Zero

    private companion object {
        const val VERTICAL_AXIS_BIAS = 1.05f
        const val STALE_RELEASE_MS = 100L
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun rememberMediaViewerTransitionState(
    pagerState: PagerState,
    openSourceRect: Rect?,
    openSourceCorners: TileCornerRadii,
    containerSize: IntSize,
    items: ImmutableList<TimelineMediaProjection>,
    activeMediaId: String,
    isZoomed: Boolean,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (String) -> Unit,
    onDismissFinished: () -> Unit,
    systemBars: ViewerSystemBars = rememberViewerSystemBars(),
): MediaViewerTransitionState {
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    val imageLoader = SingletonImageLoader.get(LocalContext.current)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val boundsRegistry = LocalTimelineTileBoundsRegistry.current
    val metrics = rememberViewerMetrics()
    val statusBarHeightPx = with(density) {
        WindowInsets.statusBarsIgnoringVisibility.getTop(density).toFloat()
    }

    val state = remember(
        scope,
        imageLoader,
        memoryCacheKeyRegistry,
        boundsRegistry,
        metrics,
        systemBars,
    ) {
        MediaViewerTransitionState(
            scope = scope,
            imageLoader = imageLoader,
            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
            boundsRegistry = boundsRegistry,
            metrics = metrics,
            initialPagerState = pagerState,
            initialSize = containerSize,
            initiallyHasOpenSource = openSourceRect != null &&
                containerSize.width > 0 &&
                containerSize.height > 0,
            systemBars = systemBars,
            onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
            onPreSyncScroll = onPreSyncScroll,
            onDismissFinished = onDismissFinished,
        )
    }

    state.pagerState = pagerState
    state.items = items
    state.activeMediaId = activeMediaId
    state.isZoomed = isZoomed
    state.openSourceRect = openSourceRect
    state.openSourceCorners = openSourceCorners
    state.onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged
    state.onPreSyncScroll = onPreSyncScroll
    state.onDismissFinished = { onDismissFinished() }
    state.setStatusBarHeight(statusBarHeightPx)

    return state
}
