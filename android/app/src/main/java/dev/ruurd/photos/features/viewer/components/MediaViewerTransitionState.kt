package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.exponentialDecay
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.lerp
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import coil3.SingletonImageLoader
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.LocalTimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.min

@Immutable
data class ClosingHeroState(
    val item: TimelineMediaProjection,
    val startRect: Rect,
    val targetRect: Rect?,
    val startBackdropAlpha: Float,
)

private enum class VerticalGestureMode {
    NONE,
    DISMISS_VIEWER,
    INFO_PANEL,
}

/**
 * Owns the viewer's hero open/close animation, the drag-to-dismiss gesture, and the unified
 * swipe-up Media Info panel that shares vertical space directly beneath the active photo/video.
 */
@Stable
class MediaViewerTransitionState(
    private val scope: CoroutineScope,
    private val imageLoader: ImageLoader,
    private val memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    private val boundsRegistry: TimelineTileBoundsRegistry,
    initialPagerState: PagerState,
    initialSize: IntSize,
    initiallyHasOpenSource: Boolean,
    private val touchSlopPx: Float,
    private val dismissThresholdPx: Float,
    private val fastFlickMinDistancePx: Float,
    private val infoCommitThresholdPx: Float,
    private val infoFlickVelocityPx: Float,
    private val systemBars: ViewerSystemBars,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (String) -> Unit,
    onInfoOpenChanged: (Boolean) -> Unit,
    onDismissFinished: () -> Unit,
) {
    /** Assigned by [rememberMediaViewerTransitionState] on every composition. */
    internal var pagerState: PagerState = initialPagerState
    internal var items: ImmutableList<TimelineMediaProjection> = persistentListOf()
    internal var activeMediaId: String = ""
    internal var isZoomed: Boolean = false
    internal var openSourceRect: Rect? = null
    internal var statusBarHeightPx: Float = 0f
    internal var onHiddenTimelineMediaChanged: (String?) -> Unit = onHiddenTimelineMediaChanged
    internal var onPreSyncScroll: (String) -> Unit = onPreSyncScroll
    internal var onInfoOpenChanged: (Boolean) -> Unit = onInfoOpenChanged
    internal var onDismissFinished: () -> Unit = onDismissFinished

    var viewerSize by mutableStateOf(initialSize)
        private set

    // Opening hero animation state
    var hasFinishedOpening by mutableStateOf(!initiallyHasOpenSource)
        private set
    private val openProgress = Animatable(if (initiallyHasOpenSource) 0f else 1f)
    private val fallbackFadeIn = Animatable(if (initiallyHasOpenSource) 1f else 0f)
    var openingHeroImageReady by mutableStateOf(false)
        private set

    // Closing hero animation state
    var closingState by mutableStateOf<ClosingHeroState?>(null)
        private set
    private val closeProgress = Animatable(0f)

    // 2D finger drag-to-dismiss state
    private var isActivelyDragging by mutableStateOf(false)
    private var rawDragOffset by mutableStateOf(Offset.Zero)
    private val settleDragOffset = Animatable(Offset.Zero, Offset.VectorConverter)
    private var settleJob: Job? = null

    // Unified swipe-up Media Info panel state (moves 1:1 with the photo/video)
    private var isDraggingInfo by mutableStateOf(false)
    private var isAnimatingInfoOpen by mutableStateOf(false)
    private var infoAnimationTargetPx by mutableFloatStateOf(0f)
    private var rawInfoTravelPx by mutableFloatStateOf(0f)
    private val settleInfoTravel = Animatable(0f)
    private var infoSettleJob: Job? = null
    private var infoContentHeightPx by mutableIntStateOf(0)

    val infoTravelPx: Float
        get() = if (isDraggingInfo) rawInfoTravelPx else settleInfoTravel.value

    val isInfoPanelActive: Boolean by derivedStateOf {
        infoTravelPx > 0.5f || isDraggingInfo || isAnimatingInfoOpen
    }

    /** True from the instant swipe-up starts until the photo settles back to its resting y-pos. */
    val shouldShowInfoPanel: Boolean by derivedStateOf {
        (infoTravelPx > 0.5f || isAnimatingInfoOpen) && !isClosing && !isOpeningHero
    }

    private val currentActiveItem: TimelineMediaProjection?
        get() = items.getOrNull(pagerState.currentPage)?.takeIf { it.id == activeMediaId }
            ?: items.firstOrNull { it.id == activeMediaId }
            ?: items.getOrNull(pagerState.currentPage)

    val infoPanelBaseTopPx: Float
        get() {
            val containerH = viewerSize.height.toFloat()
            if (containerH <= 0f || viewerSize.width <= 0) return 0f
            val item = currentActiveItem ?: return containerH * 0.7f
            return fittedRect(item).bottom.coerceIn(0f, containerH)
        }

    val settledInfoTravelPx: Float
        get() {
            val containerH = viewerSize.height.toFloat()
            if (containerH <= 0f || viewerSize.width <= 0) return 0f
            val item = currentActiveItem
            val fittedH = item?.let { fittedRect(it).height } ?: (containerH * 0.4f)
            val baseTop = infoPanelBaseTopPx
            val minSettledTop = containerH * 0.32f
            val maxSettledTop = containerH * 0.44f
            val desiredSettledTop = (statusBarHeightPx + fittedH).coerceIn(minSettledTop, maxSettledTop)
            val desiredTravel = (baseTop - desiredSettledTop).coerceAtLeast(containerH * 0.12f)
            val contentH = infoContentHeightPx.toFloat()
            if (contentH <= 0f) return desiredTravel
            val bottomAlignTravel = (baseTop + contentH - containerH).coerceAtLeast(0f)
            // When the panel's content is shorter than the remaining open space, stop once its
            // bottom reaches the bottom of the screen instead of leaving a black gap underneath.
            return if (bottomAlignTravel > 0f) {
                min(desiredTravel, bottomAlignTravel)
            } else {
                min(desiredTravel, containerH * 0.15f)
            }
        }

    private val maxInfoTravelPx: Float
        get() {
            val containerH = viewerSize.height.toFloat()
            if (containerH <= 0f) return settledInfoTravelPx
            val contentH = infoContentHeightPx.toFloat()
            if (contentH <= 0f) return settledInfoTravelPx
            return (infoPanelBaseTopPx + contentH - containerH).coerceAtLeast(settledInfoTravelPx)
        }

    val infoPanelTopPx: Float
        get() = infoPanelBaseTopPx - infoTravelPx

    val isInfoCoveringStatusBar: Boolean by derivedStateOf {
        shouldShowInfoPanel && infoPanelTopPx <= statusBarHeightPx * 0.5f
    }

    private val infoOpenFraction: Float
        get() {
            val settled = settledInfoTravelPx
            return if (settled > 0f) (infoTravelPx / settled).coerceIn(0f, 1f) else 0f
        }

    val hasValidOpenSource: Boolean
        get() = openSourceRect != null && viewerSize.width > 0 && viewerSize.height > 0

    val isOpeningHero: Boolean
        get() = hasValidOpenSource && !hasFinishedOpening && closingState == null

    val isClosing: Boolean get() = closingState != null

    val isDragging: Boolean get() = isActivelyDragging

    val dragOffset: Offset
        get() = if (isActivelyDragging) rawDragOffset else settleDragOffset.value

    val contentTranslationY: Float
        get() = if (infoTravelPx > 0f) -infoTravelPx else dragOffset.y

    private val dragDistanceY: Float get() = dragOffset.y.coerceAtLeast(0f)

    val viewerScale: Float
        get() = if (infoTravelPx > 0f) 1f else (1f - (dragDistanceY / 1400f)).coerceIn(0.55f, 1f)

    /** The pager stays invisible until the opening hero has landed (or fades in without a source tile). */
    val pagerAlpha: Float
        get() = when {
            isOpeningHero -> 0f
            hasValidOpenSource -> 1f
            else -> fallbackFadeIn.value
        }

    fun openingRect(sourceRect: Rect, item: TimelineMediaProjection): Rect = lerp(sourceRect, fittedRect(item), openProgress.value)

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

    fun closingAlpha(closing: ClosingHeroState): Float = if (closing.targetRect != null) 1f else (1f - closeProgress.value).coerceIn(0f, 1f)

    private fun fittedRect(item: TimelineMediaProjection): Rect = computeFittedRect(
        containerWidth = viewerSize.width.toFloat(),
        containerHeight = viewerSize.height.toFloat(),
        ratio = resolvePhotoRatio(item),
    )

    private val dragBackdropAlpha: Float get() = (1f - (dragDistanceY / 500f)).coerceIn(0f, 1f)

    private val chromeDragAlpha: Float get() = (1f - (dragDistanceY / 90f)).coerceIn(0f, 1f)

    val backdropAlpha: Float
        get() {
            val closing = closingState
            return when {
                closing != null ->
                    (closing.startBackdropAlpha * (1f - closeProgress.value)).coerceIn(0f, 1f)

                isOpeningHero -> openProgress.value.coerceIn(0f, 1f)

                infoTravelPx > 0f -> 1f

                hasValidOpenSource -> dragBackdropAlpha

                else -> (fallbackFadeIn.value * dragBackdropAlpha).coerceIn(0f, 1f)
            }
        }

    fun chromeAlpha(visibilityAlpha: Float): Float {
        val base = when {
            closingState != null -> 0f
            isOpeningHero -> ((openProgress.value * 1.4f) - 0.2f).coerceIn(0f, 1f) * visibilityAlpha
            hasValidOpenSource -> chromeDragAlpha * visibilityAlpha
            else -> (fallbackFadeIn.value * chromeDragAlpha) * visibilityAlpha
        }
        val infoFade = (1f - infoOpenFraction * 2.2f).coerceIn(0f, 1f)
        return base * infoFade
    }

    fun backButtonAlpha(visibilityAlpha: Float): Float {
        val normalChrome = when {
            closingState != null -> 0f
            isOpeningHero -> ((openProgress.value * 1.4f) - 0.2f).coerceIn(0f, 1f) * visibilityAlpha
            hasValidOpenSource -> chromeDragAlpha * visibilityAlpha
            else -> (fallbackFadeIn.value * chromeDragAlpha) * visibilityAlpha
        }
        if (closingState != null || isOpeningHero) return normalChrome
        return maxOf(normalChrome, infoOpenFraction)
    }

    fun onSizeChanged(size: IntSize) {
        viewerSize = size
    }

    fun onInfoContentHeightChanged(heightPx: Int) {
        if (heightPx <= 0 || heightPx == infoContentHeightPx) return
        infoContentHeightPx = heightPx
        if (isDraggingInfo || isClosing) return

        // Retarget if the panel opened (e.g. via "About") before its real content height was known.
        if (isAnimatingInfoOpen) {
            val updatedTarget = settledInfoTravelPx
            if (abs(updatedTarget - infoAnimationTargetPx) > 1f) {
                infoSettleJob?.cancel()
                infoSettleJob = scope.launch {
                    animateInfoTo(target = updatedTarget, markOpen = true)
                }
            }
        } else if (settleInfoTravel.value > maxInfoTravelPx) {
            infoSettleJob?.cancel()
            infoSettleJob = scope.launch {
                settleInfoTravel.snapTo(maxInfoTravelPx)
            }
        }
    }

    fun onHeroImageReady() {
        openingHeroImageReady = true
    }

    /** Drives the opening hero animation, or the fade-in when opened without source bounds. */
    suspend fun playOpening() {
        if (hasValidOpenSource) {
            openProgress.animateTo(
                targetValue = 1f,
                animationSpec = spring(
                    dampingRatio = 0.85f,
                    stiffness = Spring.StiffnessMediumLow,
                ),
            )
            if (!isClosing) {
                hasFinishedOpening = true
                onHiddenTimelineMediaChanged(null)
            }
        } else {
            fallbackFadeIn.animateTo(1f, tween(durationMillis = FALLBACK_FADE_MS))
        }
    }

    /** Drives the closing hero animation, then reports the viewer as dismissed. */
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
        onHiddenTimelineMediaChanged(null)
        onDismissFinished()
    }

    fun requestOpenInfoPanel() {
        if (isClosing || isOpeningHero || isZoomed || isDraggingInfo) return
        val target = settledInfoTravelPx
        if (target <= 0f) return
        if (infoTravelPx >= target - 1f && !isAnimatingInfoOpen) {
            onInfoOpenChanged(true)
            return
        }
        infoSettleJob?.cancel()
        infoSettleJob = scope.launch {
            animateInfoTo(target = target, markOpen = true)
        }
    }

    fun requestCloseInfoPanel() {
        if (isDraggingInfo) return
        if (infoTravelPx <= 0.5f && !isAnimatingInfoOpen) {
            onInfoOpenChanged(false)
            return
        }
        infoSettleJob?.cancel()
        infoSettleJob = scope.launch {
            animateInfoTo(target = 0f, markOpen = false)
        }
    }

    fun resetInfoPanelImmediate() {
        infoSettleJob?.cancel()
        infoSettleJob = null
        isDraggingInfo = false
        isAnimatingInfoOpen = false
        infoAnimationTargetPx = 0f
        rawInfoTravelPx = 0f
        scope.launch {
            settleInfoTravel.updateBounds(lowerBound = 0f, upperBound = null)
            settleInfoTravel.snapTo(0f)
        }
        onInfoOpenChanged(false)
    }

    private suspend fun animateInfoTo(
        target: Float,
        markOpen: Boolean,
        initialVelocity: Float = 0f,
    ) {
        isAnimatingInfoOpen = markOpen
        infoAnimationTargetPx = target
        try {
            settleInfoTravel.updateBounds(lowerBound = 0f, upperBound = null)
            settleInfoTravel.animateTo(
                targetValue = target,
                initialVelocity = initialVelocity,
                animationSpec = spring(
                    dampingRatio = 0.92f,
                    stiffness = INFO_PANEL_SPRING_STIFFNESS,
                ),
            )
            onInfoOpenChanged(markOpen)
        } finally {
            isAnimatingInfoOpen = false
        }
    }

    fun startClosing(
        releaseOffset: Offset,
        releaseScale: Float,
        releaseBackdropAlpha: Float,
    ) {
        if (isClosing) return
        settleJob?.cancel()
        settleJob = null
        infoSettleJob?.cancel()
        infoSettleJob = null
        isDraggingInfo = false
        isAnimatingInfoOpen = false
        infoAnimationTargetPx = 0f
        rawInfoTravelPx = 0f

        val activeItem = currentActiveItem

        if (activeItem == null || viewerSize.width <= 0 || viewerSize.height <= 0) {
            systemBars.restoreForTimeline()
            onHiddenTimelineMediaChanged(null)
            onDismissFinished()
            return
        }

        val w = viewerSize.width.toFloat()
        val h = viewerSize.height.toFloat()
        val fittedRect = computeFittedRect(w, h, resolvePhotoRatio(activeItem))

        val startRect: Rect
        val startAlpha: Float
        val sourceRect = openSourceRect
        if (isOpeningHero && sourceRect != null) {
            val t = openProgress.value
            startRect = lerp(sourceRect, fittedRect, t)
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
            startAlpha = releaseBackdropAlpha
        }

        val targetRect = boundsRegistry.boundsInRoot(activeItem.id)
        if (targetRect == null) {
            onPreSyncScroll(activeItem.id)
        }
        onHiddenTimelineMediaChanged(targetRect?.let { activeItem.id })

        systemBars.restoreForTimeline()
        isActivelyDragging = false
        closingState = ClosingHeroState(
            item = activeItem,
            startRect = startRect,
            targetRect = targetRect,
            startBackdropAlpha = startAlpha,
        )
    }

    /** The tile's ratio when known, else the ratio of the already-cached preview bitmap. */
    private fun resolvePhotoRatio(item: TimelineMediaProjection): Float {
        val baseRatio = item.ratio
        if (baseRatio.isFinite() && baseRatio > 0f && baseRatio != 1f) return baseRatio
        val cached = memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
        if (cached != null && cached.width > 0 && cached.height > 0) {
            return cached.width.toFloat() / cached.height.toFloat()
        }
        return if (baseRatio.isFinite() && baseRatio > 0f) baseRatio else 1f
    }

    /**
     * Unified vertical gesture handler on the untransformed viewer root:
     * - Swipe down from rest drags the viewer over the timeline to dismiss.
     * - Swipe up from rest fades in the info panel right below the photo and moves both 1:1.
     * - Once the info panel is open/active, both up and down drags move/scroll the photo & panel together.
     */
    fun Modifier.dismissDragModifier(): Modifier = pointerInput(Unit) {
        val velocityTracker = RecentVerticalVelocityTracker()

        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            if (isClosing || isOpeningHero || isZoomed) {
                return@awaitEachGesture
            }

            val pointerId = down.id
            var totalDx = 0f
            var totalDy = 0f
            var gestureMode = VerticalGestureMode.NONE
            val gestureStartInfoTravel = infoTravelPx
            val panelAlreadyActive = gestureStartInfoTravel > 0.5f
            var lastEventTimeMs = down.uptimeMillis

            velocityTracker.reset()
            velocityTracker.addPosition(down.uptimeMillis, down.position.y)

            try {
                while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Initial)
                    val change = event.changes.firstOrNull { it.id == pointerId } ?: break
                    lastEventTimeMs = change.uptimeMillis

                    val multiTouch = event.changes.count { it.pressed } > 1
                    if (multiTouch) break

                    if (change.pressed) {
                        velocityTracker.addPosition(change.uptimeMillis, change.position.y)
                    }

                    val dx = change.position.x - change.previousPosition.x
                    val dy = change.position.y - change.previousPosition.y
                    totalDx += dx
                    totalDy += dy

                    when (gestureMode) {
                        VerticalGestureMode.NONE -> {
                            val verticalAxisBias = if (panelAlreadyActive) 1.0f else 1.05f
                            val isVerticalDrag = abs(totalDy) > touchSlopPx &&
                                    abs(totalDy) > abs(totalDx) * verticalAxisBias

                            if (isVerticalDrag) {
                                if (panelAlreadyActive || totalDy < 0f) {
                                    gestureMode = VerticalGestureMode.INFO_PANEL
                                    infoSettleJob?.cancel()
                                    infoSettleJob = null
                                    isAnimatingInfoOpen = false
                                    val initialTravel = if (panelAlreadyActive) {
                                        (gestureStartInfoTravel - totalDy).coerceIn(0f, maxInfoTravelPx)
                                    } else {
                                        (-totalDy).coerceIn(0f, maxInfoTravelPx)
                                    }
                                    rawInfoTravelPx = initialTravel
                                    isDraggingInfo = true
                                    change.consume()
                                } else {
                                    gestureMode = VerticalGestureMode.DISMISS_VIEWER
                                    settleJob?.cancel()
                                    settleJob = null
                                    val draggedId = items.getOrNull(pagerState.currentPage)?.id
                                        ?: activeMediaId
                                    onHiddenTimelineMediaChanged(draggedId)
                                    rawDragOffset = settleDragOffset.value + Offset(totalDx, totalDy)
                                    isActivelyDragging = true
                                    change.consume()
                                }
                            } else if (abs(totalDx) > touchSlopPx) {
                                // Horizontal swipe belongs to HorizontalPager (or ignored when panel open).
                                break
                            }
                        }

                        VerticalGestureMode.DISMISS_VIEWER -> {
                            change.consume()
                            rawDragOffset += Offset(dx, dy)
                        }

                        VerticalGestureMode.INFO_PANEL -> {
                            change.consume()
                            rawInfoTravelPx = (rawInfoTravelPx - dy).coerceIn(0f, maxInfoTravelPx)
                        }
                    }

                    if (!change.pressed) break
                }
            } finally {
                when (gestureMode) {
                    VerticalGestureMode.DISMISS_VIEWER -> if (!isClosing) {
                        val releaseOffset = rawDragOffset
                        val releaseY = releaseOffset.y.coerceAtLeast(0f)
                        val velocityY = velocityTracker.calculateVelocityY(lastEventTimeMs)
                        val shouldDismiss = releaseY > dismissThresholdPx ||
                                (velocityY > 1000f && releaseY > fastFlickMinDistancePx)

                        if (shouldDismiss) {
                            val releaseScale = (1f - (releaseY / 1400f)).coerceIn(0.55f, 1f)
                            val releaseAlpha = (1f - (releaseY / 500f)).coerceIn(0f, 1f)
                            startClosing(releaseOffset, releaseScale, releaseAlpha)
                        } else {
                            settleJob?.cancel()
                            settleJob = scope.launch {
                                settleDragOffset.snapTo(releaseOffset)
                                isActivelyDragging = false
                                settleDragOffset.animateTo(
                                    targetValue = Offset.Zero,
                                    animationSpec = spring(
                                        dampingRatio = Spring.DampingRatioNoBouncy,
                                        stiffness = Spring.StiffnessMedium,
                                    ),
                                )
                                if (!isActivelyDragging && !isClosing) {
                                    onHiddenTimelineMediaChanged(null)
                                }
                            }
                        }
                    }

                    VerticalGestureMode.INFO_PANEL -> if (!isClosing) {
                        val releaseTravel = rawInfoTravelPx
                        // Positive travelVelocity means moving upward into the info panel.
                        val travelVelocity = -velocityTracker.calculateVelocityY(lastEventTimeMs)
                        val settled = settledInfoTravelPx
                        val maxTravel = maxInfoTravelPx

                        infoSettleJob?.cancel()
                        infoSettleJob = scope.launch {
                            settleInfoTravel.updateBounds(lowerBound = 0f, upperBound = null)
                            settleInfoTravel.snapTo(releaseTravel)
                            isDraggingInfo = false

                            if (releaseTravel >= settled) {
                                onInfoOpenChanged(true)
                                if (abs(travelVelocity) > 250f && maxTravel > settled) {
                                    try {
                                        settleInfoTravel.updateBounds(
                                            lowerBound = settled,
                                            upperBound = maxTravel,
                                        )
                                        settleInfoTravel.animateDecay(
                                            initialVelocity = travelVelocity,
                                            animationSpec = exponentialDecay(frictionMultiplier = 1.4f),
                                        )
                                    } catch (e: CancellationException) {
                                        throw e
                                    } finally {
                                        settleInfoTravel.updateBounds(lowerBound = 0f, upperBound = null)
                                    }
                                }
                            } else {
                                val threshold = if (gestureStartInfoTravel <= 1f) {
                                    min(settled * 0.18f, infoCommitThresholdPx)
                                } else {
                                    min(settled * 0.22f, infoCommitThresholdPx)
                                }
                                val shouldOpen = when {
                                    travelVelocity > infoFlickVelocityPx -> true
                                    travelVelocity < -infoFlickVelocityPx -> false
                                    gestureStartInfoTravel <= 1f -> releaseTravel > threshold
                                    else -> releaseTravel >= settled - threshold
                                }
                                val target = if (shouldOpen) settled else 0f
                                val directedVelocity = if (shouldOpen) {
                                    travelVelocity.coerceAtLeast(0f)
                                } else {
                                    travelVelocity.coerceAtMost(0f)
                                }
                                animateInfoTo(
                                    target = target,
                                    markOpen = shouldOpen,
                                    initialVelocity = directedVelocity,
                                )
                            }
                        }
                    }

                    VerticalGestureMode.NONE -> Unit
                }
            }
        }
    }

    private companion object {
        const val FALLBACK_FADE_MS = 180
        const val INFO_PANEL_SPRING_STIFFNESS = 1300f
    }
}

/**
 * Windowed linear vertical velocity over `pressed` move samples, avoiding quadratic polynomial
 * sign-reversals when `ACTION_UP` repeats the last coordinates or a quick flick decelerates slightly.
 */
private class RecentVerticalVelocityTracker {
    private val times = LongArray(CAPACITY)
    private val ys = FloatArray(CAPACITY)
    private var head = 0
    private var count = 0

    fun reset() {
        head = 0
        count = 0
    }

    fun addPosition(uptimeMillis: Long, y: Float) {
        if (count > 0) {
            val prevIdx = (head + count - 1) % CAPACITY
            if (times[prevIdx] == uptimeMillis) {
                ys[prevIdx] = y
                return
            }
        }
        val insertIdx = (head + count) % CAPACITY
        times[insertIdx] = uptimeMillis
        ys[insertIdx] = y
        if (count == CAPACITY) {
            head = (head + 1) % CAPACITY
        } else {
            count++
        }
    }

    /** Vertical velocity in px/sec (positive = moving downward on screen). */
    fun calculateVelocityY(releaseTimeMs: Long): Float {
        if (count < 2) return 0f
        val newestIdx = (head + count - 1) % CAPACITY
        val newestTime = times[newestIdx]
        if (releaseTimeMs - newestTime > STALE_RELEASE_MS) return 0f

        val newestY = ys[newestIdx]
        val windowStart = newestTime - WINDOW_MS
        var oldestIdx = newestIdx
        for (i in 0 until count - 1) {
            val idx = (head + i) % CAPACITY
            if (times[idx] >= windowStart) {
                oldestIdx = idx
                break
            }
        }
        if (oldestIdx == newestIdx) {
            val prevIdx = (head + count - 2) % CAPACITY
            if (newestTime - times[prevIdx] > MAX_FALLBACK_SPAN_MS) return 0f
            oldestIdx = prevIdx
        }
        val dtMs = newestTime - times[oldestIdx]
        if (dtMs <= 0L) return 0f
        return (newestY - ys[oldestIdx]) * 1000f / dtMs.toFloat()
    }

    private companion object {
        const val CAPACITY = 20
        const val WINDOW_MS = 80L
        const val STALE_RELEASE_MS = 100L
        const val MAX_FALLBACK_SPAN_MS = 150L
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun rememberMediaViewerTransitionState(
    pagerState: PagerState,
    openSourceRect: Rect?,
    containerSize: IntSize,
    items: ImmutableList<TimelineMediaProjection>,
    activeMediaId: String,
    isZoomed: Boolean,
    isChromeVisible: Boolean,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (String) -> Unit,
    onInfoOpenChanged: (Boolean) -> Unit,
    onDismissFinished: () -> Unit,
): MediaViewerTransitionState {
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    val viewConfig = LocalViewConfiguration.current
    val systemBars = rememberViewerSystemBars()
    val imageLoader = SingletonImageLoader.get(LocalContext.current)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val boundsRegistry = LocalTimelineTileBoundsRegistry.current
    val statusBarHeightPx = with(density) {
        WindowInsets.statusBarsIgnoringVisibility.getTop(density).toFloat()
    }

    val state = remember(
        scope,
        imageLoader,
        memoryCacheKeyRegistry,
        boundsRegistry,
        viewConfig,
        density,
        systemBars,
    ) {
        MediaViewerTransitionState(
            scope = scope,
            imageLoader = imageLoader,
            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
            boundsRegistry = boundsRegistry,
            initialPagerState = pagerState,
            initialSize = containerSize,
            initiallyHasOpenSource = openSourceRect != null &&
                    containerSize.width > 0 &&
                    containerSize.height > 0,
            touchSlopPx = viewConfig.touchSlop,
            dismissThresholdPx = with(density) { DISMISS_THRESHOLD_DP.dp.toPx() },
            fastFlickMinDistancePx = with(density) { FAST_FLICK_MIN_DISTANCE_DP.dp.toPx() },
            infoCommitThresholdPx = with(density) { INFO_COMMIT_THRESHOLD_DP.dp.toPx() },
            infoFlickVelocityPx = with(density) { INFO_FLICK_VELOCITY_DP_PER_SEC.dp.toPx() },
            systemBars = systemBars,
            onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
            onPreSyncScroll = onPreSyncScroll,
            onInfoOpenChanged = onInfoOpenChanged,
            onDismissFinished = onDismissFinished,
        )
    }

    ViewerSystemBarsEffect(
        systemBars = systemBars,
        isChromeVisible = isChromeVisible,
        isClosing = state.isClosing,
        isInfoVisible = state.shouldShowInfoPanel,
        isInfoCoveringStatusBar = state.isInfoCoveringStatusBar,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
    )

    state.pagerState = pagerState
    state.items = items
    state.activeMediaId = activeMediaId
    state.isZoomed = isZoomed
    state.openSourceRect = openSourceRect
    state.statusBarHeightPx = statusBarHeightPx
    state.onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged
    state.onPreSyncScroll = onPreSyncScroll
    state.onInfoOpenChanged = onInfoOpenChanged
    state.onDismissFinished = onDismissFinished

    return state
}

private const val DISMISS_THRESHOLD_DP = 110f
private const val FAST_FLICK_MIN_DISTANCE_DP = 40f
private const val INFO_COMMIT_THRESHOLD_DP = 44f
private const val INFO_FLICK_VELOCITY_DP_PER_SEC = 100f

/** The photo's rect when fitted (contained and centered) in the viewer. */
private fun computeFittedRect(containerWidth: Float, containerHeight: Float, ratio: Float): Rect {
    if (containerWidth <= 0f || containerHeight <= 0f) return Rect.Zero
    val safeRatio = if (ratio.isFinite() && ratio > 0f) ratio else 1f
    val containerRatio = containerWidth / containerHeight
    val fittedWidth: Float
    val fittedHeight: Float
    if (safeRatio > containerRatio) {
        fittedWidth = containerWidth
        fittedHeight = containerWidth / safeRatio
    } else {
        fittedHeight = containerHeight
        fittedWidth = containerHeight * safeRatio
    }
    val left = (containerWidth - fittedWidth) / 2f
    val top = (containerHeight - fittedHeight) / 2f
    return Rect(left, top, left + fittedWidth, top + fittedHeight)
}