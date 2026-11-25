package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.VolumeOff
import androidx.compose.material.icons.automirrored.filled.VolumeUp
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.domain.model.VideoPlaybackPolicy
import dev.ruurd.photos.domain.model.VideoQuality
import dev.ruurd.photos.domain.model.VideoQualityOption
import kotlinx.collections.immutable.ImmutableList

/**
 * Floating video transport controls matching the viewer layout:
 * - Row 1: Play/Pause (left), `position / duration` in `tnum` (center), optional Quality chip + Mute (right)
 * - Row 2: Full-width seekbar with a tall vertical pill thumb and buffered progress indicator
 */
@Composable
fun VideoControlsBar(
    isPlaying: Boolean,
    isMuted: Boolean,
    durationMs: Long,
    positionMsProvider: () -> Long,
    bufferedMsProvider: () -> Long,
    activeQuality: VideoQuality?,
    qualityOptions: ImmutableList<VideoQualityOption>,
    showQualitySelector: Boolean,
    onTogglePlayPause: () -> Unit,
    onToggleMute: () -> Unit,
    onSeekTo: (Long) -> Unit,
    onSelectQuality: (VideoQuality) -> Unit,
    modifier: Modifier = Modifier,
) {
    var isScrubbing by remember { mutableStateOf(false) }
    var scrubPositionMs by remember { mutableLongStateOf(0L) }

    val displayedPositionProvider = remember(positionMsProvider) {
        {
            if (isScrubbing) scrubPositionMs else positionMsProvider()
        }
    }

    Column(
        modifier = modifier
            .fillMaxWidth()
            .clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null,
                onClick = {},
            )
            .padding(horizontal = 14.dp),
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(40.dp),
        ) {
            Box(
                modifier = Modifier
                    .align(Alignment.CenterStart)
                    .size(40.dp)
                    .clip(CircleShape)
                    .clickable(onClick = onTogglePlayPause),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                    contentDescription = if (isPlaying) "Pause" else "Play",
                    tint = Color.White,
                    modifier = Modifier.size(26.dp),
                )
            }

            VideoTimeText(
                positionMsProvider = displayedPositionProvider,
                durationMs = durationMs,
                modifier = Modifier.align(Alignment.Center),
            )

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                modifier = Modifier.align(Alignment.CenterEnd),
            ) {
                if (showQualitySelector && activeQuality != null && qualityOptions.isNotEmpty()) {
                    VideoQualitySelectorButton(
                        activeQuality = activeQuality,
                        options = qualityOptions,
                        onSelectQuality = onSelectQuality,
                    )
                }

                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .clip(CircleShape)
                        .clickable(onClick = onToggleMute),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = if (isMuted) {
                            Icons.AutoMirrored.Filled.VolumeOff
                        } else {
                            Icons.AutoMirrored.Filled.VolumeUp
                        },
                        contentDescription = if (isMuted) "Unmute" else "Mute",
                        tint = Color.White,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(4.dp))

        VideoSeekBar(
            durationMs = durationMs,
            isScrubbing = isScrubbing,
            positionMsProvider = displayedPositionProvider,
            bufferedMsProvider = bufferedMsProvider,
            onScrubStart = { targetMs ->
                isScrubbing = true
                scrubPositionMs = targetMs
            },
            onScrubMove = { targetMs ->
                scrubPositionMs = targetMs
            },
            onScrubEnd = { targetMs ->
                isScrubbing = false
                onSeekTo(targetMs)
            },
            onScrubCancel = {
                isScrubbing = false
            },
        )
    }
}

@Composable
private fun VideoTimeText(
    positionMsProvider: () -> Long,
    durationMs: Long,
    modifier: Modifier = Modifier,
) {
    val timeText by remember(durationMs, positionMsProvider) {
        derivedStateOf {
            val currentLabel = VideoPlaybackPolicy.formatVideoTime(positionMsProvider())
            val totalLabel = VideoPlaybackPolicy.formatVideoTime(durationMs)
            "$currentLabel / $totalLabel"
        }
    }
    Text(
        text = timeText,
        style = MaterialTheme.typography.labelLarge.copy(
            fontFeatureSettings = "tnum",
            fontWeight = FontWeight.Medium,
            fontSize = 13.sp,
            lineHeight = 16.sp,
        ),
        color = Color.White,
        modifier = modifier,
    )
}

@Composable
private fun VideoQualitySelectorButton(
    activeQuality: VideoQuality,
    options: ImmutableList<VideoQualityOption>,
    onSelectQuality: (VideoQuality) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }

    Box(modifier = modifier) {
        Surface(
            shape = CircleShape,
            color = Color.White.copy(alpha = 0.14f),
            border = BorderStroke(1.dp, Color.White.copy(alpha = 0.16f)),
            modifier = Modifier
                .clip(CircleShape)
                .clickable { expanded = true },
        ) {
            Text(
                text = VideoPlaybackPolicy.shortQualityLabel(activeQuality),
                style = MaterialTheme.typography.labelSmall.copy(
                    fontFeatureSettings = "tnum",
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 11.sp,
                ),
                color = Color.White,
                modifier = Modifier.padding(horizontal = 9.dp, vertical = 4.dp),
            )
        }

        DropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            options.forEach { option ->
                val isSelected = option.quality == activeQuality
                DropdownMenuItem(
                    text = { Text(option.label) },
                    enabled = option.isEnabled,
                    trailingIcon = if (isSelected) {
                        {
                            Icon(
                                imageVector = Icons.Default.Check,
                                contentDescription = "Selected",
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    } else {
                        null
                    },
                    onClick = {
                        expanded = false
                        onSelectQuality(option.quality)
                    },
                )
            }
        }
    }
}

@Composable
private fun VideoSeekBar(
    durationMs: Long,
    isScrubbing: Boolean,
    positionMsProvider: () -> Long,
    bufferedMsProvider: () -> Long,
    onScrubStart: (Long) -> Unit,
    onScrubMove: (Long) -> Unit,
    onScrubEnd: (Long) -> Unit,
    onScrubCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val currentDuration by rememberUpdatedState(durationMs)
    val currentOnScrubStart by rememberUpdatedState(onScrubStart)
    val currentOnScrubMove by rememberUpdatedState(onScrubMove)
    val currentOnScrubEnd by rememberUpdatedState(onScrubEnd)
    val currentOnScrubCancel by rememberUpdatedState(onScrubCancel)

    Canvas(
        modifier = modifier
            .fillMaxWidth()
            .height(36.dp)
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Main)
                    val totalDuration = currentDuration
                    if (totalDuration <= 0L || size.width <= 0) return@awaitEachGesture

                    fun xToMs(x: Float): Long {
                        val fraction = (x / size.width.toFloat()).coerceIn(0f, 1f)
                        return (fraction * totalDuration).toLong()
                    }

                    var committed = false
                    try {
                        down.consume()
                        var latestMs = xToMs(down.position.x)
                        currentOnScrubStart(latestMs)

                        while (true) {
                            val event = awaitPointerEvent(PointerEventPass.Main)
                            val change = event.changes.firstOrNull { it.id == down.id } ?: break
                            if (!change.pressed) {
                                change.consume()
                                committed = true
                                currentOnScrubEnd(latestMs)
                                break
                            }
                            change.consume()
                            latestMs = xToMs(change.position.x)
                            currentOnScrubMove(latestMs)
                        }
                    } finally {
                        if (!committed) {
                            currentOnScrubCancel()
                        }
                    }
                }
            },
    ) {
        val totalMs = currentDuration
        val playedFraction = if (totalMs > 0L) {
            (positionMsProvider().toFloat() / totalMs.toFloat()).coerceIn(0f, 1f)
        } else {
            0f
        }
        val bufferedFraction = if (totalMs > 0L) {
            (bufferedMsProvider().toFloat() / totalMs.toFloat()).coerceIn(0f, 1f)
        } else {
            0f
        }

        val trackHeightPx = (if (isScrubbing) 6.dp else 4.dp).toPx()
        val trackTop = (size.height - trackHeightPx) / 2f
        val trackCorner = CornerRadius(trackHeightPx / 2f, trackHeightPx / 2f)

        // Unplayed track
        drawRoundRect(
            color = Color.White.copy(alpha = 0.28f),
            topLeft = Offset(0f, trackTop),
            size = Size(size.width, trackHeightPx),
            cornerRadius = trackCorner,
        )

        // Buffered track
        if (bufferedFraction > 0f) {
            drawRoundRect(
                color = Color.White.copy(alpha = 0.45f),
                topLeft = Offset(0f, trackTop),
                size = Size(size.width * bufferedFraction, trackHeightPx),
                cornerRadius = trackCorner,
            )
        }

        // Played track
        val playedWidth = size.width * playedFraction
        if (playedWidth > 0f) {
            drawRoundRect(
                color = Color.White,
                topLeft = Offset(0f, trackTop),
                size = Size(playedWidth, trackHeightPx),
                cornerRadius = trackCorner,
            )
        }

        // End dot on unplayed track (subtle visual cue matching the reference layout)
        val endDotRadius = (trackHeightPx / 2f) * 0.9f
        drawCircle(
            color = Color.White.copy(alpha = 0.6f),
            radius = endDotRadius,
            center = Offset(size.width - endDotRadius, size.height / 2f),
        )

        // Vertical pill thumb
        val thumbWidthPx = (if (isScrubbing) 5.dp else 4.dp).toPx()
        val thumbHeightPx = (if (isScrubbing) 28.dp else 24.dp).toPx()
        val maxThumbCenterX = (size.width - thumbWidthPx / 2f).coerceAtLeast(thumbWidthPx / 2f)
        val thumbCenterX = playedWidth.coerceIn(thumbWidthPx / 2f, maxThumbCenterX)
        val thumbTop = (size.height - thumbHeightPx) / 2f
        drawRoundRect(
            color = Color.White,
            topLeft = Offset(thumbCenterX - thumbWidthPx / 2f, thumbTop),
            size = Size(thumbWidthPx, thumbHeightPx),
            cornerRadius = CornerRadius(thumbWidthPx / 2f, thumbWidthPx / 2f),
        )
    }
}