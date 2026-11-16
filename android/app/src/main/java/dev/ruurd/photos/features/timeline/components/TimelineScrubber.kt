package dev.ruurd.photos.features.timeline.components

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
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
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
 */
@Composable
fun TimelineScrubber(
    layout: TimelineLayoutIndex,
    listState: LazyListState,
    viewportHeightDp: Float,
    bottomPaddingDp: Float = 0f,
    onScrubbingChanged: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current
    val textMeasurer = rememberTextMeasurer()

    val currentLayout by rememberUpdatedState(layout)
    val currentMaxScrollDp by rememberUpdatedState(
        (layout.totalHeightDp + bottomPaddingDp - viewportHeightDp).coerceAtLeast(1f)
    )
    val currentOnScrubbingChanged by rememberUpdatedState(onScrubbingChanged)

    val trackColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
    val thumbColor = MaterialTheme.colorScheme.primary
    val yearTextColor = MaterialTheme.colorScheme.onSurfaceVariant
    val yearStyle = remember(yearTextColor) {
        TextStyle(
            fontSize = 11.sp,
            fontWeight = FontWeight.Medium,
            color = yearTextColor
        )
    }
    val measuredYearMarks = remember(layout.yearFractions, yearStyle, textMeasurer) {
        layout.yearFractions.map { mark ->
            MeasuredYearMark(
                fraction = mark.fraction,
                textLayout = textMeasurer.measure(AnnotatedString(mark.year.toString()), yearStyle)
            )
        }
    }

    var isDragging by remember { mutableStateOf(false) }
    var dragFraction by remember { mutableFloatStateOf(0f) }
    var draggedLabel by remember { mutableStateOf<String?>(null) }
    var lastHapticYear by remember { mutableStateOf<Int?>(null) }
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    var pillHeightPx by remember { mutableIntStateOf(0) }
    var seekJob by remember { mutableStateOf<Job?>(null) }
    var pauseJob by remember { mutableStateOf<Job?>(null) }

    val scrollFraction by remember(layout, listState, viewportHeightDp, bottomPaddingDp) {
        derivedStateOf {
            if (layout.totalHeightDp <= 0f || layout.rows.isEmpty()) {
                0f
            } else {
                val maxScroll = (layout.totalHeightDp + bottomPaddingDp - viewportHeightDp)
                    .coerceAtLeast(1f)
                val firstIndex = listState.firstVisibleItemIndex.coerceIn(0, layout.rows.lastIndex)
                val rowOffsetDp = layout.rows[firstIndex].estimatedOffsetTopDp
                val itemScrollDp = listState.firstVisibleItemScrollOffset / density.density
                ((rowOffsetDp + itemScrollDp) / maxScroll).coerceIn(0f, 1f)
            }
        }
    }
    val fraction = if (isDragging) dragFraction else scrollFraction

    fun seek(rawFraction: Float) {
        val activeLayout = currentLayout
        if (activeLayout.isEmpty) return

        val target = rawFraction.coerceIn(0f, 1f)
        dragFraction = target

        val targetOffsetDp = target * currentMaxScrollDp
        val index = activeLayout.findRowIndexAtOffsetDp(targetOffsetDp)
        draggedLabel = activeLayout.monthLabelAt(index)

        val year = activeLayout.yearAt(index)
        if (year != null && year != lastHapticYear) {
            haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            lastHapticYear = year
        }

        val rowTopDp = activeLayout.rows.getOrNull(index)?.estimatedOffsetTopDp ?: 0f
        val withinRowOffsetPx =
            ((targetOffsetDp - rowTopDp).coerceAtLeast(0f) * density.density).roundToInt()

        seekJob?.cancel()
        seekJob = scope.launch { listState.scrollToItem(index, withinRowOffsetPx) }
    }

    Box(
        modifier = modifier
            .fillMaxHeight()
            .width(ScrubberWidth)
            .systemGestureExclusion()
            .onSizeChanged { sizePx = it }
            .pointerInput(Unit) {
                try {
                    detectVerticalDragGestures(
                        onDragStart = { offset ->
                            pauseJob?.cancel()
                            isDragging = true
                            lastHapticYear = null
                            currentOnScrubbingChanged(true)
                            seek(offset.y / size.height.coerceAtLeast(1))
                        },
                        onDragEnd = {
                            pauseJob?.cancel()
                            isDragging = false
                            draggedLabel = null
                            currentOnScrubbingChanged(false)
                        },
                        onDragCancel = {
                            pauseJob?.cancel()
                            isDragging = false
                            draggedLabel = null
                            currentOnScrubbingChanged(false)
                        },
                        onVerticalDrag = { change, _ ->
                            change.consume()
                            currentOnScrubbingChanged(true)
                            pauseJob?.cancel()
                            pauseJob = scope.launch {
                                delay(SCRUB_PAUSE_MS)
                                currentOnScrubbingChanged(false)
                            }
                            seek(change.position.y / size.height.coerceAtLeast(1))
                        }
                    )
                } finally {
                    pauseJob?.cancel()
                    isDragging = false
                    draggedLabel = null
                    currentOnScrubbingChanged(false)
                }
            }
    ) {
        Canvas(modifier = Modifier.fillMaxSize()) {
            val inset = 6.dp.toPx()
            val trackWidth = 4.dp.toPx()
            val trackX = size.width - inset - trackWidth

            drawRoundRect(
                color = trackColor,
                topLeft = Offset(trackX, 0f),
                size = Size(trackWidth, size.height),
                cornerRadius = CornerRadius(trackWidth / 2)
            )

            val thumbHeight = 44.dp.toPx()
            val thumbWidth = trackWidth + 2.dp.toPx()
            drawRoundRect(
                color = thumbColor,
                topLeft = Offset(trackX - 1.dp.toPx(), (size.height - thumbHeight) * fraction),
                size = Size(thumbWidth, thumbHeight),
                cornerRadius = CornerRadius(thumbWidth / 2)
            )

            if (isDragging) {
                var previousY = Float.NEGATIVE_INFINITY
                measuredYearMarks.forEach { mark ->
                    val textSize = mark.textLayout.size
                    val y = (size.height * mark.fraction - textSize.height / 2f)
                        .coerceIn(0f, (size.height - textSize.height).coerceAtLeast(0f))
                    if (y - previousY >= textSize.height * 1.6f) {
                        drawText(
                            textLayoutResult = mark.textLayout,
                            topLeft = Offset(trackX - 8.dp.toPx() - textSize.width, y)
                        )
                        previousY = y
                    }
                }
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

@Immutable
private data class MeasuredYearMark(
    val fraction: Float,
    val textLayout: TextLayoutResult
)

private val ScrubberWidth = 40.dp
private const val SCRUB_PAUSE_MS = 80L