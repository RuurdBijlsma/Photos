package dev.ruurd.photos.features.timeline.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.systemGestureExclusion
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
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
 * Right-edge timeline scrubber. Jumps with `scrollToItem` (never animated) while dragging, tracks the
 * thumb by row offset instead of row index so unequal row heights do not make it jitter, and excludes
 * the system back gesture so vertical drags never leave the app.
 *
 * The touch strip is always present (otherwise scrubbing could never start) but only the visuals fade
 * in while the list scrolls or the user drags. The track itself only appears during a drag.
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

    val trackColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
    val thumbColor = MaterialTheme.colorScheme.primary
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
    var dragFraction by remember { mutableFloatStateOf(0f) }
    var draggedLabel by remember { mutableStateOf<String?>(null) }
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    var pillHeightPx by remember { mutableIntStateOf(0) }
    val dragState = remember { ScrubberDragState() }

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
    val visible = isDragging || listState.isScrollInProgress
    val visualsAlpha by animateFloatAsState(
        targetValue = if (visible) 1f else 0f,
        animationSpec = tween(durationMillis = 120),
        label = "scrubber_alpha"
    )

    fun seek(rawFraction: Float) {
        val activeLayout = currentLayout
        if (activeLayout.isEmpty) return

        val target = rawFraction.coerceIn(0f, 1f)
        dragFraction = target

        val targetOffsetDp = target * currentMaxScrollDp
        val index = activeLayout.findRowIndexAtOffsetDp(targetOffsetDp)
        draggedLabel = activeLayout.monthLabelAt(index)

        val year = activeLayout.yearAt(index)
        if (year != null && year != dragState.lastHapticYear) {
            haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            dragState.lastHapticYear = year
        }

        val rowTopDp = activeLayout.rows.getOrNull(index)?.estimatedOffsetTopDp ?: 0f
        val withinRowOffsetPx =
            ((targetOffsetDp - rowTopDp).coerceAtLeast(0f) * density.density).roundToInt()

        dragState.seekJob?.cancel()
        dragState.seekJob = scope.launch { listState.scrollToItem(index, withinRowOffsetPx) }
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
                            seek(offset.y / size.height.coerceAtLeast(1))
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
                            seek(change.position.y / size.height.coerceAtLeast(1))
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

        Canvas(
            modifier = Modifier
                .fillMaxSize()
                // Dragging draws at full alpha so the layered pills are not clipped by the layer bounds.
                .alpha(if (isDragging) 1f else visualsAlpha)
        ) {
            val inset = 6.dp.toPx()
            val trackWidth = 4.dp.toPx()
            // Right edge of the screen: the box deliberately overhangs it for the clipped thumb.
            val screenRight = size.width - ScrubberOverhang.toPx()
            val trackX = screenRight - inset - trackWidth
            val thumbDiameter = 16.dp.toPx()
            // Center the thumb slightly past the right edge so ~30% of the circle is clipped off-screen.
            val thumbCenter = Offset(
                x = screenRight - thumbDiameter * 0.2f,
                y = (size.height - thumbDiameter) * fraction + thumbDiameter / 2f
            )

            if (isDragging) {
                drawRoundRect(
                    color = trackColor,
                    topLeft = Offset(trackX, 0f),
                    size = Size(trackWidth, size.height),
                    cornerRadius = CornerRadius(trackWidth / 2)
                )
                drawYearPills(
                    marks = measuredYearMarks,
                    activeYear = activeYear,
                    trackX = trackX,
                    pillColor = pillColor,
                    activePillColor = activePillColor,
                    paddingX = 6.dp.toPx(),
                    paddingY = 2.dp.toPx(),
                    gap = 4.dp.toPx()
                )
            }

            drawCircle(color = thumbColor, radius = thumbDiameter / 2f, center = thumbCenter)
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
                                x = (-56).dp.roundToPx(),
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

/** Small opaque pills next to the track, skipping any that would overlap the previous one. */
private fun DrawScope.drawYearPills(
    marks: List<MeasuredYearMark>,
    activeYear: Int?,
    trackX: Float,
    pillColor: Color,
    activePillColor: Color,
    paddingX: Float,
    paddingY: Float,
    gap: Float
) {
    var previousBottom = Float.NEGATIVE_INFINITY
    marks.forEach { mark ->
        val isActive = mark.year == activeYear
        val text = if (isActive) mark.activeTextLayout else mark.textLayout
        val textSize = text.size
        val pillHeight = textSize.height + paddingY * 2
        val pillWidth = textSize.width + paddingX * 2
        val pillTop = (size.height * mark.fraction - pillHeight / 2f)
            .coerceIn(0f, (size.height - pillHeight).coerceAtLeast(0f))

        if (pillTop >= previousBottom + gap) {
            val left = trackX - gap - pillWidth
            drawRoundRect(
                color = if (isActive) activePillColor else pillColor,
                topLeft = Offset(left, pillTop),
                size = Size(pillWidth, pillHeight),
                cornerRadius = CornerRadius(pillHeight / 2f)
            )
            drawText(textLayoutResult = text, topLeft = Offset(left + paddingX, pillTop + paddingY))
            previousBottom = pillTop + pillHeight
        }
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
    var seekJob: Job? = null
    var pauseJob: Job? = null
}

@Immutable
private data class MeasuredYearMark(
    val year: Int,
    val fraction: Float,
    val textLayout: TextLayoutResult,
    val activeTextLayout: TextLayoutResult
)

private val ScrubberWidth = 28.dp
private val ScrubberOverhang = 12.dp
private const val SCRUB_PAUSE_MS = 80L
