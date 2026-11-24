package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.DecayAnimationSpec
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.rememberSplineBasedDecay
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
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import coil3.SingletonImageLoader
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.LocalTimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.lerp
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
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
    val startCorners: TileCornerRadii,
    val targetCorners: TileCornerRadii,
    val startBackdropAlpha: Float,
)

private enum class VerticalGestureMode {
    NONE,
    DISMISS_VIEWER,
    INFO_PANEL,
}

/**
 * Owns the viewer's hero open/close animation, the drag-to-dismiss gesture, and the inline
 * swipe-up Media Info panel anchored beneath each page in the horizontal pager.
 *
 * Open progress ([infoOpenFraction]) is normalized across all items so swiping horizontally while
 * the panel is open keeps the sheet top level across differing photo aspect ratios, while
 * detail scroll overflow ([sheetScrollPx]) tracks the currently scrolled page.
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
    private val doubleTapSlopPx: Float,
    private val doubleTapTimeoutMs: Long,
    private val longPressTimeoutMs: Long,
    private val dismissThresholdPx: Float,
    private val fastFlickMinDistancePx: Float,
    private val infoCommitThresholdPx: Float,
    private val infoFlickVelocityPx: Float,
    private val minFlingVelocityPx: Float,
    private val maxFlingVelocityPx: Float,
    private val sheetCornerOverlapPx: Float,
    private val flingDecay: DecayAnimationSpec<Float>,
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
    internal var openSourceCorners: TileCornerRadii = TileCornerRadii.Zero
    internal var statusBarHeightPx: Float = 0f
    internal var onHiddenTimelineMediaChanged: (String?) -> Unit = onHiddenTimelineMediaChanged
    internal var onPreSyncScroll: (String) -> Unit = onPreSyncScroll
    internal var onInfoOpenChanged: (Boolean) -> Unit = onInfoOpenChanged
    internal var onDismissFinished: (Unit) -> Unit = { onDismissFinished() }

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

    // Unified swipe-up Media Info panel state
    var isDraggingInfo by mutableStateOf(false)
        private set
    private var isAnimatingInfoOpen by mutableStateOf(false)
    private var isClosingInfoPanel by mutableStateOf(false)
    private var infoOpenFraction by mutableFloatStateOf(0f)
    private var sheetScrollPx by mutableFloatStateOf(0f)
    private var scrolledPageId by mutableStateOf<String?>(null)
    private var rawPanelTravelPx by mutableFloatStateOf(0f)
    private val infoTravelAnim = Animatable(0f)
    private var infoSettleJob: Job? = null
    private var infoAnimToken: Long = 0L
    private val infoContentHeights = HashMap<String, Int>()

    // Tracks the previous clean single tap so double-tap + drag yields to Telephoto quick-zoom.
    private var lastTapUpTimeMs: Long = 0L
    private var lastTapUpPosition: Offset = Offset.Zero

    val isInfoPanelActive: Boolean by derivedStateOf {
        infoOpenFraction > 0.005f || sheetScrollPx > 0.5f || isDraggingInfo || isAnimatingInfoOpen
    }

    /** True from the instant swipe-up starts until the photo settles back to its resting y-pos. */
    val shouldShowInfoPanel: Boolean by derivedStateOf {
        (infoOpenFraction > 0.005f || sheetScrollPx > 0.5f || isAnimatingInfoOpen) &&
            !isClosing &&
            !isOpeningHero
    }

    private val currentActiveItem: TimelineMediaProjection?
        get() = items.getOrNull(pagerState.currentPage)?.takeIf { it.id == activeMediaId }
            ?: items.firstOrNull { it.id == activeMediaId }
            ?: items.getOrNull(pagerState.currentPage)

    val sheetRestTopPx: Float
        get() {
            val containerH = viewerSize.height.toFloat()
            if (containerH <= 0f) return 0f
            return (containerH * SHEET_REST_TOP_FRACTION).coerceAtLeast(statusBarHeightPx)
        }

    private fun baseTopPx(item: TimelineMediaProjection): Float {
        val containerH = viewerSize.height.toFloat()
        if (containerH <= 0f || viewerSize.width <= 0) return 0f
        return fittedRect(item).bottom.coerceIn(0f, containerH)
    }

    private fun sheetOverlapPx(item: TimelineMediaProjection): Float = min(
        sheetCornerOverlapPx,
        fittedRect(item).height * MAX_OVERLAP_HEIGHT_FRACTION,
    ).coerceAtLeast(0f)

    private fun sheetTravelToRestPx(item: TimelineMediaProjection): Float = (baseTopPx(item) - sheetRestTopPx).coerceAtLeast(0f)

    private fun photoTravelToRestPx(item: TimelineMediaProjection): Float = (sheetTravelToRestPx(item) - sheetOverlapPx(item)).coerceAtLeast(0f)

    private fun maxSheetScrollPx(item: TimelineMediaProjection): Float {
        val containerH = viewerSize.height.toFloat()
        if (containerH <= 0f) return 0f
        val contentH = (infoContentHeights[item.id] ?: 0).toFloat()
        if (contentH <= 0f) return 0f
        return (sheetRestTopPx + contentH - containerH).coerceAtLeast(0f)
    }

    fun pagePhotoTravelPx(item: TimelineMediaProjection): Float {
        val rest = photoTravelToRestPx(item)
        val scroll = if (scrolledPageId == item.id) sheetScrollPx else 0f
        return rest * infoOpenFraction + scroll
    }

    fun pageInfoPanelTravelPx(item: TimelineMediaProjection): Float {
        val rest = sheetTravelToRestPx(item)
        val scroll = if (scrolledPageId == item.id) sheetScrollPx else 0f
        return rest * infoOpenFraction + scroll
    }

    fun pageInfoPanelTopPx(item: TimelineMediaProjection): Float = baseTopPx(item) - pageInfoPanelTravelPx(item)

    val isInfoCoveringStatusBar: Boolean by derivedStateOf {
        val activeTop = currentActiveItem?.let(::pageInfoPanelTopPx) ?: sheetRestTopPx
        shouldShowInfoPanel && activeTop <= statusBarHeightPx * 0.5f
    }

    val hasValidOpenSource: Boolean
        get() = openSourceRect != null && viewerSize.width > 0 && viewerSize.height > 0

    val isOpeningHero: Boolean
        get() = hasValidOpenSource && !hasFinishedOpening && closingState == null

    val isClosing: Boolean get() = closingState != null

    val isDragging: Boolean get() = isActivelyDragging

    val dragOffset: Offset
        get() = if (isActivelyDragging) rawDragOffset else settleDragOffset.value

    private val dragDistanceY: Float get() = dragOffset.y.coerceAtLeast(0f)

    val viewerScale: Float
        get() = if (infoOpenFraction > 0f || sheetScrollPx > 0f) {
            1f
        } else {
            (1f - (dragDistanceY / 1400f)).coerceIn(0.55f, 1f)
        }

    /** The pager stays invisible until the opening hero has landed (or fades in without a source tile). */
    val pagerAlpha: Float
        get() = when {
            isOpeningHero -> 0f
            hasValidOpenSource -> 1f
            else -> fallbackFadeIn.value
        }

    fun openingRect(sourceRect: Rect, item: TimelineMediaProjection): Rect = lerp(sourceRect, fittedRect(item), openProgress.value)

    fun openingCornerRadii(): TileCornerRadii = lerp(openSourceCorners, TileCornerRadii.Zero, openProgress.value)

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

                infoOpenFraction > 0f || sheetScrollPx > 0f -> 1f

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

    fun onInfoContentHeightChanged(mediaId: String, heightPx: Int) {
        if (heightPx <= 0 || infoContentHeights[mediaId] == heightPx) return
        infoContentHeights[mediaId] = heightPx
        if (isDraggingInfo || isClosing || scrolledPageId != mediaId) return

        val item = items.firstOrNull { it.id == mediaId } ?: return
        val maxScroll = maxSheetScrollPx(item)
        if (sheetScrollPx > maxScroll) {
            sheetScrollPx = maxScroll
        }
    }

    fun onHorizontalPageSettled(mediaId: String) {
        if (scrolledPageId != null && scrolledPageId != mediaId && !isDraggingInfo) {
            infoAnimToken++
            infoSettleJob?.cancel()
            infoSettleJob = null
            sheetScrollPx = 0f
            scrolledPageId = null
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
        onDismissFinished(Unit)
    }

    fun requestOpenInfoPanel() {
        if (isClosing || isOpeningHero || isZoomed || isDraggingInfo) return
        if (infoOpenFraction >= 0.999f && !isAnimatingInfoOpen) {
            onInfoOpenChanged(true)
            return
        }
        if (isAnimatingInfoOpen) return
        val item = currentActiveItem ?: return
        val target = sheetTravelToRestPx(item)
        if (target <= 0f) return
        val token = ++infoAnimToken
        infoSettleJob?.cancel()
        infoSettleJob = scope.launch {
            animateInfoTo(
                token = token,
                item = item,
                startTravel = pageInfoPanelTravelPx(item),
                targetTravel = target,
                markOpen = true,
            )
        }
    }

    fun requestCloseInfoPanel() {
        if (isDraggingInfo || isClosingInfoPanel) return
        if (infoOpenFraction <= 0.005f && sheetScrollPx <= 0.5f && !isAnimatingInfoOpen) {
            onInfoOpenChanged(false)
            return
        }
        val item = currentActiveItem ?: run {
            resetInfoPanelImmediate()
            return
        }
        val token = ++infoAnimToken
        infoSettleJob?.cancel()
        infoSettleJob = scope.launch {
            animateInfoTo(
                token = token,
                item = item,
                startTravel = pageInfoPanelTravelPx(item),
                targetTravel = 0f,
                markOpen = false,
            )
        }
    }

    fun resetInfoPanelImmediate() {
        infoAnimToken++
        infoSettleJob?.cancel()
        infoSettleJob = null
        isDraggingInfo = false
        isAnimatingInfoOpen = false
        isClosingInfoPanel = false
        infoOpenFraction = 0f
        sheetScrollPx = 0f
        scrolledPageId = null
        rawPanelTravelPx = 0f
        scope.launch {
            infoTravelAnim.updateBounds(lowerBound = 0f, upperBound = null)
            infoTravelAnim.snapTo(0f)
        }
        onInfoOpenChanged(false)
    }

    private fun applyTravel(item: TimelineMediaProjection, travelPx: Float) {
        val clamped = travelPx.coerceAtLeast(0f)
        val rest = sheetTravelToRestPx(item)
        if (rest <= 0f) {
            infoOpenFraction = if (clamped > 0f) 1f else 0f
            sheetScrollPx = clamped.coerceIn(0f, maxSheetScrollPx(item))
        } else if (clamped <= rest) {
            infoOpenFraction = (clamped / rest).coerceIn(0f, 1f)
            sheetScrollPx = 0f
        } else {
            infoOpenFraction = 1f
            sheetScrollPx = (clamped - rest).coerceIn(0f, maxSheetScrollPx(item))
        }
        scrolledPageId = if (sheetScrollPx > 0.5f) item.id else null
    }

    private suspend fun animateInfoTo(
        token: Long,
        item: TimelineMediaProjection,
        startTravel: Float,
        targetTravel: Float,
        markOpen: Boolean,
        initialVelocity: Float = 0f,
    ) {
        isAnimatingInfoOpen = markOpen
        isClosingInfoPanel = !markOpen
        try {
            infoTravelAnim.updateBounds(lowerBound = 0f, upperBound = null)
            infoTravelAnim.snapTo(startTravel)
            infoTravelAnim.animateTo(
                targetValue = targetTravel,
                initialVelocity = initialVelocity,
                animationSpec = spring(
                    dampingRatio = 0.92f,
                    stiffness = INFO_PANEL_SPRING_STIFFNESS,
                ),
            ) {
                if (token == infoAnimToken) {
                    applyTravel(item, value)
                }
            }
            if (token == infoAnimToken) {
                applyTravel(item, targetTravel)
                onInfoOpenChanged(markOpen)
            }
        } finally {
            if (token == infoAnimToken) {
                isAnimatingInfoOpen = false
                isClosingInfoPanel = false
            }
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
        infoAnimToken++
        infoSettleJob?.cancel()
        infoSettleJob = null
        isDraggingInfo = false
        isAnimatingInfoOpen = false
        isClosingInfoPanel = false
        infoOpenFraction = 0f
        sheetScrollPx = 0f
        scrolledPageId = null
        rawPanelTravelPx = 0f

        val activeItem = currentActiveItem

        if (activeItem == null || viewerSize.width <= 0 || viewerSize.height <= 0) {
            systemBars.restoreForTimeline()
            onHiddenTimelineMediaChanged(null)
            onDismissFinished(Unit)
            return
        }

        val w = viewerSize.width.toFloat()
        val h = viewerSize.height.toFloat()
        val fittedRect = computeFittedRect(w, h, resolvePhotoRatio(activeItem))

        val startRect: Rect
        val startCorners: TileCornerRadii
        val startAlpha: Float
        val sourceRect = openSourceRect
        if (isOpeningHero && sourceRect != null) {
            val t = openProgress.value
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
        isActivelyDragging = false
        closingState = ClosingHeroState(
            item = activeItem,
            startRect = startRect,
            targetRect = targetRect,
            startCorners = startCorners,
            targetCorners = targetCorners,
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

            val panelAlreadyActive = infoOpenFraction > 0.005f || sheetScrollPx > 0.5f
            val isQuickZoomCandidate = !panelAlreadyActive &&
                lastTapUpTimeMs > 0L &&
                (down.uptimeMillis - lastTapUpTimeMs) in 0L..doubleTapTimeoutMs &&
                (down.position - lastTapUpPosition).getDistance() <= doubleTapSlopPx
            if (isQuickZoomCandidate) {
                // Consume the tap memory and yield the entire second press + drag to Telephoto.
                lastTapUpTimeMs = 0L
                return@awaitEachGesture
            }

            // Catch an ongoing info-panel fling right under the finger so touching stops momentum
            // immediately without jumping once the drag passes touch slop.
            if (infoSettleJob?.isActive == true &&
                !isAnimatingInfoOpen &&
                !isClosingInfoPanel &&
                panelAlreadyActive
            ) {
                infoAnimToken++
                infoSettleJob?.cancel()
                infoSettleJob = null
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
                            maxDisplacementPx <= touchSlopPx &&
                            (change.uptimeMillis - down.uptimeMillis) < longPressTimeoutMs
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
                            val isVerticalDrag = abs(totalDy) > touchSlopPx &&
                                abs(totalDy) > abs(totalDx) * VERTICAL_AXIS_BIAS

                            if (isVerticalDrag) {
                                lastTapUpTimeMs = 0L
                                if (panelAlreadyActive || totalDy < 0f) {
                                    val targetItem = currentActiveItem
                                    if (targetItem != null) {
                                        gestureMode = VerticalGestureMode.INFO_PANEL
                                        gestureItem = targetItem
                                        infoAnimToken++
                                        infoSettleJob?.cancel()
                                        infoSettleJob = null
                                        isAnimatingInfoOpen = false
                                        isClosingInfoPanel = false
                                        val baseTravel = pageInfoPanelTravelPx(targetItem)
                                        val initialDelta = if (panelAlreadyActive) -dy else -totalDy
                                        val maxTravel = sheetTravelToRestPx(targetItem) +
                                            maxSheetScrollPx(targetItem)
                                        rawPanelTravelPx = (baseTravel + initialDelta)
                                            .coerceIn(0f, maxTravel)
                                        applyTravel(targetItem, rawPanelTravelPx)
                                        isDraggingInfo = true
                                        change.consume()
                                    }
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
                            } else if (abs(totalDx) > touchSlopPx && abs(totalDx) >= abs(totalDy)) {
                                lastTapUpTimeMs = 0L
                                // Horizontal swipe belongs to HorizontalPager (open or closed).
                                break
                            }
                        }

                        VerticalGestureMode.DISMISS_VIEWER -> {
                            change.consume()
                            rawDragOffset += Offset(dx, dy)
                        }

                        VerticalGestureMode.INFO_PANEL -> {
                            change.consume()
                            val targetItem = gestureItem
                            if (targetItem != null) {
                                val maxTravel = sheetTravelToRestPx(targetItem) +
                                    maxSheetScrollPx(targetItem)
                                rawPanelTravelPx = (rawPanelTravelPx - dy).coerceIn(0f, maxTravel)
                                applyTravel(targetItem, rawPanelTravelPx)
                            }
                        }
                    }
                }
            } finally {
                val velocityY = if (lastEventTimeMs - lastSampleTimeMs > STALE_RELEASE_MS) {
                    0f
                } else {
                    velocityTracker
                        .calculateVelocity(Velocity(maxFlingVelocityPx, maxFlingVelocityPx))
                        .y
                }

                when (gestureMode) {
                    VerticalGestureMode.DISMISS_VIEWER -> if (!isClosing) {
                        val releaseOffset = rawDragOffset
                        val releaseY = releaseOffset.y.coerceAtLeast(0f)
                        val shouldDismiss = !isZoomed && (
                            releaseY > dismissThresholdPx ||
                                (velocityY > 1000f && releaseY > fastFlickMinDistancePx)
                            )

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

                    VerticalGestureMode.INFO_PANEL -> {
                        val targetItem = gestureItem
                        if (isClosing || targetItem == null) {
                            isDraggingInfo = false
                        } else if (isZoomed) {
                            isDraggingInfo = false
                            val token = ++infoAnimToken
                            infoSettleJob?.cancel()
                            infoSettleJob = scope.launch {
                                animateInfoTo(
                                    token = token,
                                    item = targetItem,
                                    startTravel = rawPanelTravelPx,
                                    targetTravel = 0f,
                                    markOpen = false,
                                )
                            }
                        } else {
                            val releaseTravel = rawPanelTravelPx
                            // Positive travelVelocity means moving upward into the info panel.
                            val travelVelocity = -velocityY
                            val rest = sheetTravelToRestPx(targetItem)
                            val maxTravel = rest + maxSheetScrollPx(targetItem)

                            val token = ++infoAnimToken
                            infoSettleJob?.cancel()
                            infoSettleJob = scope.launch {
                                isDraggingInfo = false

                                if (releaseTravel >= rest) {
                                    onInfoOpenChanged(true)
                                    if (abs(travelVelocity) > minFlingVelocityPx && maxTravel > rest) {
                                        try {
                                            infoTravelAnim.updateBounds(
                                                lowerBound = rest,
                                                upperBound = maxTravel,
                                            )
                                            infoTravelAnim.snapTo(releaseTravel)
                                            infoTravelAnim.animateDecay(
                                                initialVelocity = travelVelocity,
                                                animationSpec = flingDecay,
                                            ) {
                                                if (token == infoAnimToken) {
                                                    applyTravel(targetItem, value)
                                                }
                                            }
                                        } finally {
                                            infoTravelAnim.updateBounds(
                                                lowerBound = 0f,
                                                upperBound = null,
                                            )
                                        }
                                    }
                                } else {
                                    val threshold = if (gestureStartTravel <= 1f) {
                                        min(rest * 0.18f, infoCommitThresholdPx)
                                    } else {
                                        min(rest * 0.22f, infoCommitThresholdPx)
                                    }
                                    val shouldOpen = when {
                                        travelVelocity > infoFlickVelocityPx -> true
                                        travelVelocity < -infoFlickVelocityPx -> false
                                        gestureStartTravel <= 1f -> releaseTravel > threshold
                                        else -> releaseTravel >= rest - threshold
                                    }
                                    val target = if (shouldOpen) rest else 0f
                                    val directedVelocity = if (shouldOpen) {
                                        travelVelocity.coerceAtLeast(0f)
                                    } else {
                                        travelVelocity.coerceAtMost(0f)
                                    }
                                    animateInfoTo(
                                        token = token,
                                        item = targetItem,
                                        startTravel = releaseTravel,
                                        targetTravel = target,
                                        markOpen = shouldOpen,
                                        initialVelocity = directedVelocity,
                                    )
                                }
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
        const val SHEET_REST_TOP_FRACTION = 0.38f
        const val MAX_OVERLAP_HEIGHT_FRACTION = 0.5f
        const val VERTICAL_AXIS_BIAS = 1.05f
        const val STALE_RELEASE_MS = 100L
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun rememberMediaViewerTransitionState(
    pagerState: PagerState,
    openSourceRect: Rect?,
    openSourceCorners: TileCornerRadii,
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
    val flingDecay = rememberSplineBasedDecay<Float>()
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
        flingDecay,
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
            doubleTapSlopPx = with(density) { DOUBLE_TAP_SLOP_DP.dp.toPx() },
            doubleTapTimeoutMs = viewConfig.doubleTapTimeoutMillis,
            longPressTimeoutMs = viewConfig.longPressTimeoutMillis,
            dismissThresholdPx = with(density) { DISMISS_THRESHOLD_DP.dp.toPx() },
            fastFlickMinDistancePx = with(density) { FAST_FLICK_MIN_DISTANCE_DP.dp.toPx() },
            infoCommitThresholdPx = with(density) { INFO_COMMIT_THRESHOLD_DP.dp.toPx() },
            infoFlickVelocityPx = with(density) { INFO_FLICK_VELOCITY_DP_PER_SEC.dp.toPx() },
            minFlingVelocityPx = with(density) { MIN_FLING_VELOCITY_DP_PER_SEC.dp.toPx() },
            maxFlingVelocityPx = viewConfig.maximumFlingVelocity,
            sheetCornerOverlapPx = with(density) { PhotosRadii.Sheet.toPx() },
            flingDecay = flingDecay,
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
    state.openSourceCorners = openSourceCorners
    state.statusBarHeightPx = statusBarHeightPx
    state.onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged
    state.onPreSyncScroll = onPreSyncScroll
    state.onInfoOpenChanged = onInfoOpenChanged
    state.onDismissFinished = { onDismissFinished() }

    return state
}

private const val DOUBLE_TAP_SLOP_DP = 100f
private const val DISMISS_THRESHOLD_DP = 110f
private const val FAST_FLICK_MIN_DISTANCE_DP = 40f
private const val INFO_COMMIT_THRESHOLD_DP = 44f
private const val INFO_FLICK_VELOCITY_DP_PER_SEC = 100f
private const val MIN_FLING_VELOCITY_DP_PER_SEC = 50f

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
