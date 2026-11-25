package dev.ruurd.photos.features.viewer.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.min

/**
 * The inline swipe-up Media Info panel: how far it is open, how far its content is scrolled, and the
 * animations that settle it after a release.
 *
 * Open progress ([openFraction]) is normalized across all items so swiping horizontally while the
 * panel is open keeps the sheet top level across differing photo aspect ratios, while the detail
 * scroll overflow ([scrollPx]) tracks the currently scrolled page.
 */
@Stable
internal class ViewerInfoPanelState(
    private val scope: CoroutineScope,
    private val geometry: ViewerGeometry,
    private val metrics: ViewerMetrics,
) {
    var isDragging by mutableStateOf(false)
        private set

    /** A programmatic open that had to wait for the page to stop zooming or finish opening. */
    var isRequested by mutableStateOf(false)
        private set

    private var isAnimatingOpen by mutableStateOf(false)
    private var isClosingPanel by mutableStateOf(false)
    private var openFraction by mutableFloatStateOf(0f)
    private var scrollPx by mutableFloatStateOf(0f)
    private var scrolledPageId by mutableStateOf<String?>(null)
    private var rawTravelPx by mutableFloatStateOf(0f)
    private val travelAnim = Animatable(0f)
    private var settleJob: Job? = null
    private var animToken: Long = 0L

    /** True while the panel is open or moving, including a drag. */
    val isActive: Boolean by derivedStateOf {
        openFraction > 0.005f || scrollPx > 0.5f || isDragging || isAnimatingOpen
    }

    /** True while the panel is open, opening, or scrolled, ignoring drags that never commit. */
    val isOpenOrAnimating: Boolean by derivedStateOf {
        openFraction > 0.005f || scrollPx > 0.5f || isAnimatingOpen
    }

    /** Exact open fraction, `0f` when fully closed. */
    val openProgress: Float get() = openFraction

    /** Whether the panel has moved past the "is it there" threshold, used for gesture routing. */
    val isOpenBeyondThreshold: Boolean get() = openFraction > 0.005f || scrollPx > 0.5f

    /** Whether the panel has moved at all, used for the backdrop and scaling decisions. */
    val isOpenAnyAmount: Boolean get() = openFraction > 0f || scrollPx > 0f

    fun pagePhotoTravelPx(item: TimelineMediaProjection): Float = geometry.photoTravelToRestPx(item) * openFraction + scrollOf(item)

    fun pageTravelPx(item: TimelineMediaProjection): Float = geometry.sheetTravelToRestPx(item) * openFraction + scrollOf(item)

    fun pageTopPx(item: TimelineMediaProjection): Float = geometry.baseTopPx(item) - pageTravelPx(item)

    private fun scrollOf(item: TimelineMediaProjection): Float = if (scrolledPageId == item.id) scrollPx else 0f

    private fun maxTravelPx(item: TimelineMediaProjection): Float = geometry.sheetTravelToRestPx(item) + geometry.maxSheetScrollPx(item)

    /** Defers an open until the page stops zooming or the hero finishes flying in. */
    fun deferRequest() {
        isRequested = true
    }

    fun requestOpen(item: TimelineMediaProjection?) {
        isRequested = false
        if (openFraction >= 0.999f && !isAnimatingOpen) return
        if (isAnimatingOpen) return
        val target = item ?: return
        val rest = geometry.sheetTravelToRestPx(target)
        if (rest <= 0f) return
        val token = ++animToken
        settleJob?.cancel()
        settleJob = scope.launch {
            animateTo(
                token = token,
                item = target,
                startTravel = pageTravelPx(target),
                targetTravel = rest,
                markOpen = true,
            )
        }
    }

    fun requestClose(item: TimelineMediaProjection?) {
        if (isDragging || isClosingPanel) return
        isRequested = false
        if (openFraction <= 0.005f && scrollPx <= 0.5f && !isAnimatingOpen) return
        val target = item ?: run {
            resetImmediate()
            return
        }
        val token = ++animToken
        settleJob?.cancel()
        settleJob = scope.launch {
            animateTo(
                token = token,
                item = target,
                startTravel = pageTravelPx(target),
                targetTravel = 0f,
                markOpen = false,
            )
        }
    }

    fun resetImmediate() {
        animToken++
        settleJob?.cancel()
        settleJob = null
        isDragging = false
        isAnimatingOpen = false
        isClosingPanel = false
        openFraction = 0f
        scrollPx = 0f
        scrolledPageId = null
        rawTravelPx = 0f
        isRequested = false
        scope.launch {
            travelAnim.updateBounds(lowerBound = 0f, upperBound = null)
            travelAnim.snapTo(0f)
        }
    }

    /** Keeps the scroll offset inside the panel once its measured content height changes. */
    fun clampScroll(mediaId: String, item: TimelineMediaProjection?) {
        if (isDragging || item == null || scrolledPageId != mediaId) return
        val maxScroll = geometry.maxSheetScrollPx(item)
        if (scrollPx > maxScroll) {
            scrollPx = maxScroll
        }
    }

    fun onHorizontalPageSettled(mediaId: String) {
        if (scrolledPageId != null && scrolledPageId != mediaId && !isDragging) {
            animToken++
            settleJob?.cancel()
            settleJob = null
            scrollPx = 0f
            scrolledPageId = null
        }
    }

    /**
     * Catches an in-flight panel fling right under the finger so touching stops momentum
     * immediately instead of jumping once the drag passes touch slop.
     */
    fun stopFlingUnderFinger() {
        if (settleJob?.isActive == true && !isAnimatingOpen && !isClosingPanel) {
            animToken++
            settleJob?.cancel()
            settleJob = null
        }
    }

    fun beginDrag(
        item: TimelineMediaProjection,
        panelAlreadyActive: Boolean,
        dy: Float,
        totalDy: Float,
    ) {
        animToken++
        settleJob?.cancel()
        settleJob = null
        isAnimatingOpen = false
        isClosingPanel = false
        val baseTravel = pageTravelPx(item)
        val initialDelta = if (panelAlreadyActive) -dy else -totalDy
        rawTravelPx = (baseTravel + initialDelta).coerceIn(0f, maxTravelPx(item))
        applyTravel(item, rawTravelPx)
        isDragging = true
    }

    fun dragBy(item: TimelineMediaProjection, dy: Float) {
        rawTravelPx = (rawTravelPx - dy).coerceIn(0f, maxTravelPx(item))
        applyTravel(item, rawTravelPx)
    }

    /** Drops an in-progress drag without opening or closing, e.g. when the viewer starts closing. */
    fun cancelDrag() {
        isDragging = false
    }

    /** Animates a dragged panel back closed, e.g. when the user zooms mid-drag. */
    fun closeFromDrag(item: TimelineMediaProjection) {
        isDragging = false
        val token = ++animToken
        settleJob?.cancel()
        settleJob = scope.launch {
            animateTo(
                token = token,
                item = item,
                startTravel = rawTravelPx,
                targetTravel = 0f,
                markOpen = false,
            )
        }
    }

    /** Decides open/closed after a released drag and animates (or flings) there. */
    fun settleAfterDrag(
        item: TimelineMediaProjection,
        velocityY: Float,
        gestureStartTravelPx: Float,
    ) {
        val releaseTravel = rawTravelPx
        // Positive travelVelocity means moving upward into the info panel.
        val travelVelocity = -velocityY
        val rest = geometry.sheetTravelToRestPx(item)
        val maxTravel = maxTravelPx(item)

        val token = ++animToken
        settleJob?.cancel()
        settleJob = scope.launch {
            isDragging = false

            if (releaseTravel >= rest) {
                if (abs(travelVelocity) > metrics.minFlingVelocityPx && maxTravel > rest) {
                    try {
                        travelAnim.updateBounds(lowerBound = rest, upperBound = maxTravel)
                        travelAnim.snapTo(releaseTravel)
                        travelAnim.animateDecay(
                            initialVelocity = travelVelocity,
                            animationSpec = metrics.flingDecay,
                        ) {
                            if (token == animToken) {
                                applyTravel(item, value)
                            }
                        }
                    } finally {
                        travelAnim.updateBounds(lowerBound = 0f, upperBound = null)
                    }
                }
            } else {
                val threshold = if (gestureStartTravelPx <= 1f) {
                    min(rest * 0.18f, metrics.infoCommitThresholdPx)
                } else {
                    min(rest * 0.22f, metrics.infoCommitThresholdPx)
                }
                val shouldOpen = when {
                    travelVelocity > metrics.infoFlickVelocityPx -> true
                    travelVelocity < -metrics.infoFlickVelocityPx -> false
                    gestureStartTravelPx <= 1f -> releaseTravel > threshold
                    else -> releaseTravel >= rest - threshold
                }
                val target = if (shouldOpen) rest else 0f
                val directedVelocity = if (shouldOpen) {
                    travelVelocity.coerceAtLeast(0f)
                } else {
                    travelVelocity.coerceAtMost(0f)
                }
                animateTo(
                    token = token,
                    item = item,
                    startTravel = releaseTravel,
                    targetTravel = target,
                    markOpen = shouldOpen,
                    initialVelocity = directedVelocity,
                )
            }
        }
    }

    private fun applyTravel(item: TimelineMediaProjection, travelPx: Float) {
        val clamped = travelPx.coerceAtLeast(0f)
        val rest = geometry.sheetTravelToRestPx(item)
        if (rest <= 0f) {
            openFraction = if (clamped > 0f) 1f else 0f
            scrollPx = clamped.coerceIn(0f, geometry.maxSheetScrollPx(item))
        } else if (clamped <= rest) {
            openFraction = (clamped / rest).coerceIn(0f, 1f)
            scrollPx = 0f
        } else {
            openFraction = 1f
            scrollPx = (clamped - rest).coerceIn(0f, geometry.maxSheetScrollPx(item))
        }
        scrolledPageId = if (scrollPx > 0.5f) item.id else null
    }

    private suspend fun animateTo(
        token: Long,
        item: TimelineMediaProjection,
        startTravel: Float,
        targetTravel: Float,
        markOpen: Boolean,
        initialVelocity: Float = 0f,
    ) {
        isAnimatingOpen = markOpen
        isClosingPanel = !markOpen
        try {
            travelAnim.updateBounds(lowerBound = 0f, upperBound = null)
            travelAnim.snapTo(startTravel)
            travelAnim.animateTo(
                targetValue = targetTravel,
                initialVelocity = initialVelocity,
                animationSpec = spring(
                    dampingRatio = 0.92f,
                    stiffness = INFO_PANEL_SPRING_STIFFNESS,
                ),
            ) {
                if (token == animToken) {
                    applyTravel(item, value)
                }
            }
            if (token == animToken) {
                applyTravel(item, targetTravel)
            }
        } finally {
            if (token == animToken) {
                isAnimatingOpen = false
                isClosingPanel = false
            }
        }
    }
}

private const val INFO_PANEL_SPRING_STIFFNESS = 1300f
