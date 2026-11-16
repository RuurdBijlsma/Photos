package dev.ruurd.photos.features.timeline.components

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.systemGestureExclusion
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.UnfoldMore
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/**
 * Right-edge timeline scrubber. Jumps synchronously with `requestScrollToItem` while dragging,
 * tracks the thumb by row offset instead of row index so unequal row heights do not make it jitter,
 * and excludes the system back gesture so vertical drags never leave the app.
 *
 * The touch strip is always present (otherwise scrubbing could never start) while the thumb slides
 * in horizontally from the right when scrolling/dragging and slides back out after a short delay.
 */
@Composable
fun TimelineScrubber(
    layout: TimelineLayoutIndex,
    listState: LazyListState,
    viewportHeightDp: Float,
    bottomPaddingDp: Float = 0f,
    topPaddingDp: Float = 0f,
    onScrubbingChanged: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current
    val textMeasurer = rememberTextMeasurer()

    val currentLayout by rememberUpdatedState(layout)
    val currentMaxScrollDp by rememberUpdatedState(maxScrollDp(layout, viewportHeightDp, topPaddingDp, bottomPaddingDp))
    val currentOnScrubbingChanged by rememberUpdatedState(onScrubbingChanged)

    val thumbColor = MaterialTheme.colorScheme.surfaceContainerHigh
    val thumbIconColor = MaterialTheme.colorScheme.onSurface
    val thumbBorderColor = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.35f)
    val pillColor = MaterialTheme.colorScheme.surfaceContainerHighest
    val pillTextColor = MaterialTheme.colorScheme.onSurface
    val activePillColor = MaterialTheme.colorScheme.primary
    val activePillTextColor = MaterialTheme.colorScheme.onPrimary
    val yearStyle = remember(pillTextColor) { yearLabelStyle(pillTextColor) }
    val activeYearStyle = remember(activePillTextColor) { yearLabelStyle(activePillTextColor) }
    val measuredYearMarks = remember(layout.yearFractions, yearStyle, activeYearStyle, textMeasurer) {
        layout.yearFractions.map { mark ->
            MeasuredYearMark(
                year = mark.year,
                fraction = mark.fraction,
                textLayout = textMeasurer.measure(AnnotatedString(mark.year.toString()), yearStyle),
                activeTextLayout = textMeasurer.measure(AnnotatedString(mark.year.toString()), activeYearStyle)
            )
        }
    }

    var isDragging by remember { mutableStateOf(false) }
    var isScrubberVisible by remember { mutableStateOf(false) }
    var dragFraction by remember { mutableFloatStateOf(0f) }
    var draggedLabel by remember { mutableStateOf<String?>(null) }
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    var pillHeightPx by remember { mutableIntStateOf(0) }
    val dragState = remember { ScrubberDragState() }

    LaunchedEffect(listState) {
        var lastIndex = listState.firstVisibleItemIndex
        var lastOffset = listState.firstVisibleItemScrollOffset
        var hideJob: Job? = null

        snapshotFlow {
            ScrollObservation(
                isDragging = isDragging,
                isScrolling = listState.isScrollInProgress,
                firstIndex = listState.firstVisibleItemIndex,
                firstOffset = listState.firstVisibleItemScrollOffset
            )
        }.collect { obs ->
            val moved = obs.firstIndex != lastIndex || obs.firstOffset != lastOffset
            lastIndex = obs.firstIndex
            lastOffset = obs.firstOffset

            // Require actual position movement for list scrolls so boundary overscroll (e.g.
            // pulling down at the top) does not pop the scrubber in.
            if (obs.isDragging || (obs.isScrolling && moved)) {
                hideJob?.cancel()
                hideJob = null
                isScrubberVisible = true
            } else if (!obs.isDragging && !obs.isScrolling && isScrubberVisible && hideJob?.isActive != true) {
                hideJob = launch {
                    delay(SCRUBBER_HIDE_DELAY_MS)
                    isScrubberVisible = false
                }
            }
        }
    }

    val scrollFraction by remember(layout, listState, viewportHeightDp, topPaddingDp, bottomPaddingDp) {
        derivedStateOf {
            val maxScroll = maxScrollDp(layout, viewportHeightDp, topPaddingDp, bottomPaddingDp)
            if (maxScroll <= 1f || layout.rows.isEmpty()) {
                0f
            } else {
                val firstIndex = listState.firstVisibleItemIndex.coerceIn(0, layout.rows.lastIndex)
                val rowOffsetDp = layout.rows[firstIndex].estimatedOffsetTopDp
                val itemScrollDp = listState.firstVisibleItemScrollOffset / density.density
                ((rowOffsetDp + itemScrollDp) / maxScroll).coerceIn(0f, 1f)
            }
        }
    }
    val fraction = if (isDragging) dragFraction else scrollFraction
    val showVisuals = isDragging || isScrubberVisible
    val revealProgress by animateFloatAsState(
        targetValue = if (showVisuals) 1f else 0f,
        animationSpec = if (showVisuals) {
            spring(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = Spring.StiffnessMedium)
        } else {
            tween(durationMillis = 240)
        },
        label = "scrubber_slide"
    )
    val slideOutPx = with(density) { (ThumbVisibleWidth + 12.dp).toPx() }

    fun seek(rawFraction: Float, triggerYearHaptic: Boolean) {
        val activeLayout = currentLayout
        if (activeLayout.isEmpty) return

        val target = rawFraction.coerceIn(0f, 1f)
        dragFraction = target

        val targetOffsetDp = target * currentMaxScrollDp
        val index = activeLayout.findRowIndexAtOffsetDp(targetOffsetDp)
        draggedLabel = activeLayout.monthLabelAt(index)

        val year = activeLayout.yearAt(index)
        if (year != null && year != dragState.lastHapticYear) {
            if (triggerYearHaptic && dragState.lastHapticYear != null) {
                haptic.performHapticFeedback(HapticFeedbackType.ContextClick)
            }
            dragState.lastHapticYear = year
        }

        val rowTopDp = activeLayout.rows.getOrNull(index)?.estimatedOffsetTopDp ?: 0f
        val withinRowOffsetPx =
            ((targetOffsetDp - rowTopDp).coerceAtLeast(0f) * density.density).roundToInt()

        listState.requestScrollToItem(index, withinRowOffsetPx)
    }

    Box(
        modifier = modifier
            .fillMaxHeight()
            // Extra width hangs off-screen so the partially clipped thumb stays inside the layout.
            .width(ScrubberWidth + ScrubberOverhang)
            .offset(x = ScrubberOverhang)
            .systemGestureExclusion()
            .onSizeChanged { sizePx = it }
            .pointerInput(listState, density) {
                try {
                    detectVerticalDragGestures(
                        onDragStart = { offset ->
                            dragState.pauseJob?.cancel()
                            isDragging = true
                            dragState.lastHapticYear = null
                            currentOnScrubbingChanged(true)
                            haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
                            seek(
                                rawFraction = offset.y / size.height.coerceAtLeast(1),
                                triggerYearHaptic = false
                            )
                        },
                        onDragEnd = {
                            dragState.pauseJob?.cancel()
                            isDragging = false
                            draggedLabel = null
                            currentOnScrubbingChanged(false)
                        },
                        onDragCancel = {
                            dragState.pauseJob?.cancel()
                            isDragging = false
                            draggedLabel = null
                            currentOnScrubbingChanged(false)
                        },
                        onVerticalDrag = { change, _ ->
                            change.consume()
                            currentOnScrubbingChanged(true)
                            dragState.pauseJob?.cancel()
                            dragState.pauseJob = scope.launch {
                                delay(SCRUB_PAUSE_MS)
                                currentOnScrubbingChanged(false)
                            }
                            seek(
                                rawFraction = change.position.y / size.height.coerceAtLeast(1),
                                triggerYearHaptic = true
                            )
                        }
                    )
                } finally {
                    dragState.pauseJob?.cancel()
                    isDragging = false
                    draggedLabel = null
                    currentOnScrubbingChanged(false)
                }
            }
    ) {
        val activeYear = if (isDragging) {
            layout.yearAt(layout.findRowIndexAtOffsetDp(dragFraction * currentMaxScrollDp))
        } else {
            null
        }

        Canvas(modifier = Modifier.fillMaxSize()) {
            if (isDragging) {
                val screenRight = size.width - ScrubberOverhang.toPx()
                // Right edge of the pills: just left of the thumb so the finger cannot cover them.
                val pillRight = screenRight - ThumbVisibleWidth.toPx() - PillGap.toPx()
                drawYearPills(
                    marks = measuredYearMarks,
                    activeYear = activeYear,
                    rightEdge = pillRight,
                    pillColor = pillColor,
                    activePillColor = activePillColor,
                    paddingX = 6.dp.toPx(),
                    paddingY = 2.dp.toPx(),
                    gap = 4.dp.toPx()
                )
            }
        }

        val thumbSizePx = with(density) { ThumbDiameter.roundToPx() }
        // Left of the visible part of the circle: the rest hangs past the screen edge.
        val thumbLeftPx = sizePx.width -
                with(density) { ScrubberOverhang.roundToPx() + ThumbVisibleWidth.roundToPx() }
        Surface(
            shape = CircleShape,
            color = thumbColor,
            tonalElevation = 4.dp,
            shadowElevation = 6.dp,
            border = BorderStroke(1.dp, thumbBorderColor),
            modifier = Modifier
                .align(Alignment.TopStart)
                .offset {
                    IntOffset(
                        x = thumbLeftPx,
                        y = ((sizePx.height - thumbSizePx) * fraction).roundToInt()
                    )
                }
                .graphicsLayer {
                    val effectiveReveal = if (isDragging) 1f else revealProgress
                    translationX = (1f - effectiveReveal) * slideOutPx
                }
                .size(ThumbDiameter)
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    imageVector = Icons.Filled.UnfoldMore,
                    contentDescription = null,
                    tint = thumbIconColor,
                    modifier = Modifier
                        .offset(x = -ThumbIconOpticalShift)
                        .size(ThumbIconSize)
                )
            }
        }

        if (isDragging) {
            draggedLabel?.let { label ->
                val fallbackHeightPx = with(density) { 32.dp.roundToPx() }
                val effectivePillHeightPx = if (pillHeightPx > 0) pillHeightPx else fallbackHeightPx
                val maxPillY = (sizePx.height - effectivePillHeightPx).coerceAtLeast(0)
                val pillYPx = (maxPillY * dragFraction).roundToInt()

                Text(
                    text = label,
                    maxLines = 1,
                    softWrap = false,
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontFamily = MontserratFontFamily,
                        fontWeight = FontWeight.SemiBold
                    ),
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .wrapContentWidth(unbounded = true, align = Alignment.End)
                        .onSizeChanged { pillHeightPx = it.height }
                        .offset {
                            IntOffset(
                                // Same right edge as the year pills, so nothing hides under the finger.
                                x = -with(density) {
                                    (ScrubberOverhang + ThumbVisibleWidth + PillGap).roundToPx()
                                },
                                y = pillYPx
                            )
                        }
                        .background(
                            color = MaterialTheme.colorScheme.primaryContainer,
                            shape = RoundedCornerShape(
                                topStart = 16.dp,
                                bottomStart = 16.dp,
                                topEnd = 4.dp,
                                bottomEnd = 4.dp
                            )
                        )
                        .padding(horizontal = 12.dp, vertical = 6.dp)
                )
            }
        }
    }
}

internal data class PlacedYearMark(
    val top: Float,
    val height: Float
)

/**
 * Selects which year marks to show while reserving the oldest year (last in list) so intermediate
 * recent-ish years (e.g. 2005) never hide the oldest year in the library (e.g. 1997).
 */
internal fun selectVisibleYearMarkIndices(
    marks: List<PlacedYearMark>,
    gap: Float
): List<Int> {
    if (marks.isEmpty()) return emptyList()
    if (marks.size == 1) return listOf(0)

    val first = marks.first()
    val lastIndex = marks.lastIndex
    val last = marks[lastIndex]

    val firstBottom = first.top + first.height
    if (last.top < firstBottom + gap) {
        return listOf(0)
    }

    val selected = ArrayList<Int>(marks.size)
    selected.add(0)
    var previousBottom = firstBottom

    for (i in 1 until lastIndex) {
        val mark = marks[i]
        if (mark.top >= previousBottom + gap && mark.top + mark.height + gap <= last.top) {
            selected.add(i)
            previousBottom = mark.top + mark.height
        }
    }

    selected.add(lastIndex)
    return selected
}

/** Small opaque pills left of the thumb, keeping both ends of the year range visible. */
private fun DrawScope.drawYearPills(
    marks: List<MeasuredYearMark>,
    activeYear: Int?,
    rightEdge: Float,
    pillColor: Color,
    activePillColor: Color,
    paddingX: Float,
    paddingY: Float,
    gap: Float
) {
    if (marks.isEmpty()) return

    val placed = marks.map { mark ->
        val isActive = mark.year == activeYear
        val text = if (isActive) mark.activeTextLayout else mark.textLayout
        val pillHeight = text.size.height + paddingY * 2
        val pillTop = (size.height * mark.fraction - pillHeight / 2f)
            .coerceIn(0f, (size.height - pillHeight).coerceAtLeast(0f))
        PlacedYearMark(top = pillTop, height = pillHeight)
    }

    val visibleIndices = selectVisibleYearMarkIndices(placed, gap)
    for (i in visibleIndices) {
        val mark = marks[i]
        val placement = placed[i]
        val isActive = mark.year == activeYear
        val text = if (isActive) mark.activeTextLayout else mark.textLayout
        val pillWidth = text.size.width + paddingX * 2
        val left = rightEdge - pillWidth

        drawRoundRect(
            color = if (isActive) activePillColor else pillColor,
            topLeft = Offset(left, placement.top),
            size = Size(pillWidth, placement.height),
            cornerRadius = CornerRadius(placement.height / 2f)
        )
        drawText(
            textLayoutResult = text,
            topLeft = Offset(left + paddingX, placement.top + paddingY)
        )
    }
}

private fun maxScrollDp(
    layout: TimelineLayoutIndex,
    viewportHeightDp: Float,
    topPaddingDp: Float,
    bottomPaddingDp: Float
): Float = (layout.totalHeightDp + topPaddingDp + bottomPaddingDp - viewportHeightDp)
    .coerceAtLeast(1f)

private fun yearLabelStyle(color: Color) = TextStyle(
    fontSize = 12.sp,
    fontWeight = FontWeight.SemiBold,
    color = color
)

private class ScrubberDragState {
    var lastHapticYear: Int? = null
    var pauseJob: Job? = null
}

private data class ScrollObservation(
    val isDragging: Boolean,
    val isScrolling: Boolean,
    val firstIndex: Int,
    val firstOffset: Int
)

@Immutable
private data class MeasuredYearMark(
    val year: Int,
    val fraction: Float,
    val textLayout: TextLayoutResult,
    val activeTextLayout: TextLayoutResult
)

private val ScrubberWidth = 36.dp
private val ScrubberOverhang = 20.dp
private val ThumbDiameter = 52.dp
private val ThumbIconSize = 28.dp

/** Visible part of the thumb, i.e. everything left of the screen edge (35% hangs past it). */
private val ThumbVisibleWidth = ThumbDiameter * 0.67f
private val ThumbIconOpticalShift = 4.dp
private val PillGap = 6.dp
private const val SCRUB_PAUSE_MS = 80L
private const val SCRUBBER_HIDE_DELAY_MS = 1_500L