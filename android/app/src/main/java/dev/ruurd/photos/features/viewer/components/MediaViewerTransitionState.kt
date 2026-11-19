package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
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
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
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
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.abs

@Immutable
data class ClosingHeroState(
    val item: TimelineMediaProjection,
    val startRect: Rect,
    val targetRect: Rect?,
    val startBackdropAlpha: Float
)

/**
 * Owns the viewer's hero open/close animation, the drag-to-dismiss gesture and the alpha ramp
 * derived from them. Modeled like [dev.ruurd.photos.features.main.components.PullToRefreshState]:
 * one `@Stable` holder instead of a dozen loose `remember` slots, with the latest inputs assigned on
 * every composition.
 */
@Stable
class MediaViewerTransitionState(
    private val scope: CoroutineScope,
    private val imageLoader: ImageLoader,
    private val memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    private val boundsRegistry: TimelineTileBoundsRegistry,    private val pagerState: PagerState,
    initialSize: IntSize,
    initiallyHasOpenSource: Boolean,
    private val touchSlopPx: Float,
    private val dismissThresholdPx: Float,
    private val fastFlickMinDistancePx: Float,
    private val systemBars: ViewerSystemBars,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (String) -> Unit,
    onDismissFinished: () -> Unit
) {
    /** Assigned by [rememberMediaViewerTransitionState] on every composition. */
    internal var items: ImmutableList<TimelineMediaProjection> = persistentListOf()
    internal var activeMediaId: String = ""
    internal var isZoomed: Boolean = false
    internal var openSourceRect: Rect? = null

    private val onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged
    private val onPreSyncScroll = onPreSyncScroll
    private val onDismissFinished = onDismissFinished

    var viewerSize by mutableStateOf(initialSize)
        private set

    // Opening hero animation state
    var hasFinishedOpening by mutableStateOf(!initiallyHasOpenSource)
        private set
    val openProgress = Animatable(if (initiallyHasOpenSource) 0f else 1f)
    val fallbackFadeIn = Animatable(if (initiallyHasOpenSource) 1f else 0f)
    var openingHeroImageReady by mutableStateOf(false)
        private set

    // Closing hero animation state
    var closingState by mutableStateOf<ClosingHeroState?>(null)
        private set
    val closeProgress = Animatable(0f)

    // 2D finger drag-to-dismiss state
    private var isActivelyDragging by mutableStateOf(false)
    private var rawDragOffset by mutableStateOf(Offset.Zero)
    private val settleDragOffset = Animatable(Offset.Zero, Offset.VectorConverter)
    private var settleJob: Job? = null

    val hasValidOpenSource: Boolean
        get() = openSourceRect != null && viewerSize.width > 0 && viewerSize.height > 0

    val isOpeningHero: Boolean
        get() = hasValidOpenSource && !hasFinishedOpening && closingState == null

    val isClosing: Boolean get() = closingState != null

    val isDragging: Boolean get() = isActivelyDragging

    val dragOffset: Offset
        get() = if (isActivelyDragging) rawDragOffset else settleDragOffset.value

    private val dragDistanceY: Float get() = dragOffset.y.coerceAtLeast(0f)

    val viewerScale: Float get() = (1f - (dragDistanceY / 1400f)).coerceIn(0.55f, 1f)

    /** The pager stays invisible until the opening hero has landed (or fades in without a source tile). */
    val pagerAlpha: Float
        get() = when {
            isOpeningHero -> 0f
            hasValidOpenSource -> 1f
            else -> fallbackFadeIn.value
        }

    fun fittedRect(item: TimelineMediaProjection): Rect =
        computeFittedRect(
            containerWidth = viewerSize.width.toFloat(),
            containerHeight = viewerSize.height.toFloat(),
            ratio = resolvePhotoRatio(item)
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
                hasValidOpenSource -> dragBackdropAlpha
                else -> (fallbackFadeIn.value * dragBackdropAlpha).coerceIn(0f, 1f)
            }
        }

    fun chromeAlpha(isChromeVisible: Boolean): Float = when {
        closingState != null -> 0f
        !isChromeVisible -> 0f
        isOpeningHero -> ((openProgress.value * 1.4f) - 0.2f).coerceIn(0f, 1f)
        hasValidOpenSource -> chromeDragAlpha
        else -> (fallbackFadeIn.value * chromeDragAlpha).coerceIn(0f, 1f)
    }

    fun onSizeChanged(size: IntSize) {
        viewerSize = size
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
                    stiffness = Spring.StiffnessMediumLow
                )
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
                    stiffness = Spring.StiffnessMediumLow
                )
            )
        } else {
            closeProgress.animateTo(1f, tween(durationMillis = FALLBACK_FADE_MS))
        }
        onHiddenTimelineMediaChanged(null)
        onDismissFinished()
    }

    fun startClosing(
        releaseOffset: Offset,
        releaseScale: Float,
        releaseBackdropAlpha: Float
    ) {
        if (isClosing) return
        settleJob?.cancel()
        settleJob = null

        val activeItem = items.getOrNull(pagerState.currentPage)?.takeIf { it.id == activeMediaId }
            ?: items.firstOrNull { it.id == activeMediaId }
            ?: items.getOrNull(pagerState.currentPage)

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
            // Reversing mid-open starts right from the current interpolated hero rect.
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
                bottom = centerY + startH / 2f
            )
            startAlpha = releaseBackdropAlpha
        }

        val targetRect = boundsRegistry.boundsInRoot(activeItem.id)
        onPreSyncScroll(activeItem.id)
        onHiddenTimelineMediaChanged(targetRect?.let { activeItem.id })

        systemBars.restoreForTimeline()
        isActivelyDragging = false
        closingState = ClosingHeroState(
            item = activeItem,
            startRect = startRect,
            targetRect = targetRect,
            startBackdropAlpha = startAlpha
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
     * Two-axis drag: downward wins over the pager, horizontal swipes stay with the pager. Must be
     * applied to the untransformed root so pointer positions are stable screen coordinates.
     */
    fun dismissDragModifier(): Modifier = Modifier.pointerInput(Unit) {
        val velocityTracker = VelocityTracker()

        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            if (isClosing || isOpeningHero || isZoomed) {
                return@awaitEachGesture
            }
            val pointerId = down.id
            var totalDx = 0f
            var totalDy = 0f
            var isDraggingToDismiss = false
            velocityTracker.resetTracking()
            velocityTracker.addPosition(down.uptimeMillis, down.position)

            try {
                while (true) {
                    val pass = if (isDraggingToDismiss) {
                        PointerEventPass.Initial
                    } else {
                        PointerEventPass.Main
                    }
                    val event = awaitPointerEvent(pass)
                    val change = event.changes.firstOrNull { it.id == pointerId } ?: break

                    val multiTouch = event.changes.count { it.pressed } > 1
                    if (multiTouch) break
                    if (!isDraggingToDismiss && change.isConsumed) break

                    val dx = change.position.x - change.previousPosition.x
                    val dy = change.position.y - change.previousPosition.y
                    totalDx += dx
                    totalDy += dy

                    if (!isDraggingToDismiss) {
                        if (abs(totalDy) > touchSlopPx &&
                            abs(totalDy) > abs(totalDx) * 1.25f &&
                            totalDy > 0f
                        ) {
                            isDraggingToDismiss = true
                            settleJob?.cancel()
                            settleJob = null
                            val draggedId = items.getOrNull(pagerState.currentPage)?.id
                                ?: activeMediaId
                            onHiddenTimelineMediaChanged(draggedId)
                            rawDragOffset = settleDragOffset.value + Offset(totalDx, totalDy)
                            isActivelyDragging = true
                            velocityTracker.addPosition(change.uptimeMillis, change.position)
                            change.consume()
                        } else if (abs(totalDx) > touchSlopPx ||
                            (abs(totalDy) > touchSlopPx && totalDy < 0f)
                        ) {
                            // Horizontal swipe belongs to HorizontalPager; upward swipe is not dismiss.
                            break
                        }
                    } else {
                        velocityTracker.addPosition(change.uptimeMillis, change.position)
                        change.consume()
                        rawDragOffset += Offset(dx, dy)
                    }

                    if (!change.pressed) break
                }
            } finally {
                if (isDraggingToDismiss && !isClosing) {
                    val releaseOffset = rawDragOffset
                    val releaseY = releaseOffset.y.coerceAtLeast(0f)
                    val velocityY = velocityTracker.calculateVelocity().y
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
                                    stiffness = Spring.StiffnessMedium
                                )
                            )
                            if (!isActivelyDragging && !isClosing) {
                                onHiddenTimelineMediaChanged(null)
                            }
                        }
                    }
                }
            }
        }
    }

    private companion object {
        const val FALLBACK_FADE_MS = 180
    }
}

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
    onDismissFinished: () -> Unit
): MediaViewerTransitionState {
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    val viewConfig = LocalViewConfiguration.current
    val systemBars = rememberViewerSystemBars()
    val imageLoader = SingletonImageLoader.get(LocalContext.current)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val boundsRegistry = LocalTimelineTileBoundsRegistry.current

    val state = remember(
        pagerState,
        imageLoader,
        memoryCacheKeyRegistry,
        boundsRegistry,
        viewConfig,
        systemBars,
        onHiddenTimelineMediaChanged,
        onPreSyncScroll,
        onDismissFinished
    ) {
        MediaViewerTransitionState(
            scope = scope,
            imageLoader = imageLoader,
            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
            boundsRegistry = boundsRegistry,
            pagerState = pagerState,
            initialSize = containerSize,
            initiallyHasOpenSource = openSourceRect != null &&
                containerSize.width > 0 &&
                containerSize.height > 0,
            touchSlopPx = viewConfig.touchSlop,
            dismissThresholdPx = with(density) { DISMISS_THRESHOLD_DP.dp.toPx() },
            fastFlickMinDistancePx = with(density) { FAST_FLICK_MIN_DISTANCE_DP.dp.toPx() },
            systemBars = systemBars,
            onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
            onPreSyncScroll = onPreSyncScroll,
            onDismissFinished = onDismissFinished
        )
    }

    ViewerSystemBarsEffect(
        systemBars = systemBars,
        isChromeVisible = isChromeVisible,
        isClosing = state.isClosing,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged
    )

    state.items = items
    state.activeMediaId = activeMediaId
    state.isZoomed = isZoomed
    state.openSourceRect = openSourceRect

    return state
}

private const val DISMISS_THRESHOLD_DP = 110f
private const val FAST_FLICK_MIN_DISTANCE_DP = 40f

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
