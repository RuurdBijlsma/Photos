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
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.systemGestureExclusion
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/**
 * Right-edge timeline scrubber. Jumps with `scrollToItem` (never animated) while dragging, tracks the
 * thumb by row offset instead of row index so unequal row heights do not make it jitter, and excludes
 * the system back gesture so vertical drags never leave the app.
 */
@Composable
fun TimelineScrubber(
    layout: TimelineLayoutIndex,
    listState: LazyListState,
    onScrubbingChanged: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current
    val textMeasurer = rememberTextMeasurer()

    val trackColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
    val thumbColor = MaterialTheme.colorScheme.primary
    val yearStyle = TextStyle(
        fontSize = 11.sp,
        fontWeight = FontWeight.Medium,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )

    var isDragging by remember { mutableStateOf(false) }
    var dragFraction by remember { mutableStateOf(0f) }
    var draggedLabel by remember { mutableStateOf<String?>(null) }
    var lastHapticYear by remember { mutableStateOf<Int?>(null) }
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    var seekJob by remember { mutableStateOf<Job?>(null) }

    val scrollFraction by remember(layout, listState) {
        derivedStateOf {
            if (layout.totalHeightDp <= 0f || layout.rows.isEmpty()) {
                0f
            } else {
                val offset = layout.rows[listState.firstVisibleItemIndex.coerceIn(0, layout.rows.lastIndex)]
                    .estimatedOffsetTopDp
                (offset / layout.totalHeightDp).coerceIn(0f, 1f)
            }
        }
    }
    val fraction = if (isDragging) dragFraction else scrollFraction

    fun seek(rawFraction: Float) {
        val target = rawFraction.coerceIn(0f, 1f)
        dragFraction = target
        val index = layout.findRowIndexAtPercentage(target)
        draggedLabel = layout.monthLabelAt(index)

        val year = layout.yearAt(index)
        if (year != null && year != lastHapticYear) {
            haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            lastHapticYear = year
        }

        seekJob?.cancel()
        seekJob = scope.launch { listState.scrollToItem(index) }
    }

    Box(
        modifier = modifier
            .fillMaxHeight()
            .width(ScrubberWidth)
            .systemGestureExclusion()
            .onSizeChanged { sizePx = it }
            .pointerInput(layout) {
                detectVerticalDragGestures(
                    onDragStart = { offset ->
                        isDragging = true
                        lastHapticYear = null
                        onScrubbingChanged(true)
                        seek(offset.y / size.height.coerceAtLeast(1))
                    },
                    onDragEnd = {
                        isDragging = false
                        draggedLabel = null
                        onScrubbingChanged(false)
                    },
                    onDragCancel = {
                        isDragging = false
                        draggedLabel = null
                        onScrubbingChanged(false)
                    },
                    onVerticalDrag = { change, _ ->
                        seek(change.position.y / size.height.coerceAtLeast(1))
                    }
                )
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
                layout.yearFractions.forEach { mark ->
                    val measured = textMeasurer.measure(AnnotatedString(mark.year.toString()), yearStyle)
                    val y = (size.height * mark.fraction - measured.size.height / 2f)
                        .coerceIn(0f, size.height - measured.size.height)
                    if (y - previousY >= measured.size.height * 1.6f) {
                        drawText(
                            textLayoutResult = measured,
                            topLeft = Offset(trackX - 8.dp.toPx() - measured.size.width, y)
                        )
                        previousY = y
                    }
                }
            }
        }

        if (isDragging) {
            draggedLabel?.let { label ->
                val pillY = (sizePx.height - with(density) { 28.dp.toPx() }) * dragFraction
                Text(
                    text = label,
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontFamily = MontserratFontFamily,
                        fontWeight = FontWeight.SemiBold
                    ),
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .offset(x = (-56).dp, y = with(density) { pillY.toDp() })
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

private val ScrubberWidth = 40.dp
