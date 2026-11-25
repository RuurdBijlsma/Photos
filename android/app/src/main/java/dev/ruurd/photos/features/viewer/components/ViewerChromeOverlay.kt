package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.RotateRight
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Repeat
import androidx.compose.material.icons.outlined.Speed
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaInfoFormat
import dev.ruurd.photos.domain.model.VideoPlaybackPolicy
import dev.ruurd.photos.domain.model.VideoQuality
import dev.ruurd.photos.domain.model.VideoQualityOption
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.collections.immutable.ImmutableList

@Immutable
data class ViewerVideoChromeState(
    val isPlaying: Boolean,
    val isMuted: Boolean,
    val isLoopEnabled: Boolean,
    val playbackSpeed: Float,
    val durationMs: Long,
    val positionMsProvider: () -> Long,
    val bufferedMsProvider: () -> Long,
    val activeQuality: VideoQuality?,
    val qualityOptions: ImmutableList<VideoQualityOption>,
    val showQualitySelector: Boolean,
    val onTogglePlayPause: () -> Unit,
    val onToggleMute: () -> Unit,
    val onSeekTo: (Long) -> Unit,
    val onSelectQuality: (VideoQuality) -> Unit,
    val onSelectSpeed: (Float) -> Unit,
    val onToggleLoop: () -> Unit,
)

/**
 * Floating viewer chrome (top bar, overflow menu, bottom action pill, and video controls) plus
 * status/bottom scrims. Anchored to the screen edges; the Back button can remain visible
 * independently while the info panel is open.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ViewerChromeOverlay(
    alpha: Float,
    currentItem: TimelineMediaProjection?,
    currentDetails: MediaDetails?,
    currentLocalInfo: LocalMediaInfo?,
    isShareEnabled: Boolean,
    isCopyEnabled: Boolean,
    onBack: () -> Unit,
    onAbout: () -> Unit,
    onCopy: () -> Unit,
    onShare: () -> Unit,
    modifier: Modifier = Modifier,
    backButtonAlpha: Float = alpha,
    isSharing: Boolean = false,
    isCopying: Boolean = false,
    videoChromeState: ViewerVideoChromeState? = null,
) {
    // Ignoring visibility keeps the top bar anchored when statusBars() animates back in on tap.
    val statusBarPadding = WindowInsets.statusBarsIgnoringVisibility
        .asPaddingValues()
        .calculateTopPadding()
    val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    val topTitle = remember(currentItem, currentDetails, currentLocalInfo) {
        MediaInfoFormat.viewerTopTitle(currentDetails, currentItem, currentLocalInfo)
    }
    val topSubtitle = remember(currentItem, currentDetails, currentLocalInfo) {
        MediaInfoFormat.viewerTopSubtitle(currentDetails, currentItem, currentLocalInfo)
    }

    Box(modifier = modifier.fillMaxSize()) {
        if (alpha > 0.01f) {
            ViewerStatusBarScrim(
                statusBarHeight = statusBarPadding,
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .graphicsLayer { this.alpha = alpha },
            )
        }

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .align(Alignment.TopCenter)
                .padding(top = statusBarPadding + 6.dp, start = 8.dp, end = 8.dp),
        ) {
            if (backButtonAlpha > 0.01f) {
                ViewerCircleButton(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "Back",
                    onClick = onBack,
                    modifier = Modifier.graphicsLayer { this.alpha = backButtonAlpha },
                )
            }

            Spacer(modifier = Modifier.width(8.dp))

            Box(
                modifier = Modifier.weight(1f),
                contentAlignment = Alignment.Center,
            ) {
                if (alpha > 0.01f && (topTitle != null || topSubtitle != null)) {
                    Surface(
                        shape = CircleShape,
                        color = ViewerPillColor,
                        border = ViewerPillBorder,
                        modifier = Modifier
                            .graphicsLayer { this.alpha = alpha }
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                                onClick = {},
                            ),
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(
                                horizontal = 14.dp,
                                vertical = if (topSubtitle != null) 6.dp else 9.dp,
                            ),
                        ) {
                            topTitle?.let { title ->
                                Text(
                                    text = title,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    textAlign = TextAlign.Center,
                                    style = MaterialTheme.typography.labelLarge.copy(
                                        fontFamily = MontserratFontFamily,
                                        fontWeight = FontWeight.SemiBold,
                                        fontSize = 13.sp,
                                        lineHeight = 16.sp,
                                    ),
                                    color = Color.White,
                                )
                            }
                            topSubtitle?.let { subtitle ->
                                Text(
                                    text = subtitle,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    textAlign = TextAlign.Center,
                                    style = MaterialTheme.typography.labelSmall.copy(
                                        fontWeight = FontWeight.Medium,
                                        fontSize = 11.sp,
                                        lineHeight = 14.sp,
                                    ),
                                    color = Color.White.copy(alpha = 0.8f),
                                )
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.width(8.dp))

            if (alpha > 0.01f) {
                var isMenuExpanded by remember { mutableStateOf(false) }
                var showSpeedDialog by remember { mutableStateOf(false) }
                Box(modifier = Modifier.graphicsLayer { this.alpha = alpha }) {
                    ViewerCircleButton(
                        imageVector = Icons.Default.MoreVert,
                        contentDescription = "Options",
                        onClick = { isMenuExpanded = true },
                    )

                    DropdownMenu(
                        expanded = isMenuExpanded,
                        onDismissRequest = { isMenuExpanded = false },
                    ) {
                        DropdownMenuItem(
                            text = { Text("About") },
                            leadingIcon = {
                                Icon(
                                    imageVector = Icons.Outlined.Info,
                                    contentDescription = null,
                                )
                            },
                            onClick = {
                                isMenuExpanded = false
                                onAbout()
                            },
                        )
                        if (currentItem?.isVideo == true && videoChromeState != null) {
                            DropdownMenuItem(
                                text = { Text("Playback speed") },
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.Outlined.Speed,
                                        contentDescription = null,
                                    )
                                },
                                trailingIcon = {
                                    Text(
                                        text = VideoPlaybackPolicy.formatSpeed(videoChromeState.playbackSpeed),
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                },
                                onClick = {
                                    isMenuExpanded = false
                                    showSpeedDialog = true
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Loop video") },
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.Outlined.Repeat,
                                        contentDescription = null,
                                    )
                                },
                                trailingIcon = if (videoChromeState.isLoopEnabled) {
                                    {
                                        Icon(
                                            imageVector = Icons.Default.Check,
                                            contentDescription = "Enabled",
                                            modifier = Modifier.size(18.dp),
                                        )
                                    }
                                } else {
                                    null
                                },
                                onClick = {
                                    isMenuExpanded = false
                                    videoChromeState.onToggleLoop()
                                },
                            )
                        }
                        if (currentItem != null && !currentItem.isVideo && isCopyEnabled) {
                            DropdownMenuItem(
                                text = { Text(if (isCopying) "Copying…" else "Copy") },
                                enabled = !isCopying,
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.Outlined.ContentCopy,
                                        contentDescription = null,
                                    )
                                },
                                onClick = {
                                    isMenuExpanded = false
                                    onCopy()
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Rotate clockwise") },
                                // Wired in Phase 2D.
                                enabled = false,
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Outlined.RotateRight,
                                        contentDescription = null,
                                    )
                                },
                                onClick = { isMenuExpanded = false },
                            )
                        }
                        if (currentItem?.cloudId != null) {
                            DropdownMenuItem(
                                text = { Text("Reprocess") },
                                // Wired in Phase 2D.
                                enabled = false,
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.Outlined.Refresh,
                                        contentDescription = null,
                                    )
                                },
                                onClick = { isMenuExpanded = false },
                            )
                        }
                    }
                }

                if (showSpeedDialog && videoChromeState != null) {
                    PlaybackSpeedDialog(
                        currentSpeed = videoChromeState.playbackSpeed,
                        onSelectSpeed = videoChromeState.onSelectSpeed,
                        onDismiss = { showSpeedDialog = false },
                    )
                }
            }
        }

        if (alpha > 0.01f) {
            if (currentItem?.isVideo == true && videoChromeState != null) {
                ViewerBottomControlsScrim(
                    navBarHeight = navBarPadding,
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .graphicsLayer { this.alpha = alpha },
                )
            }

            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = navBarPadding + 14.dp)
                    .graphicsLayer { this.alpha = alpha },
            ) {
                if (currentItem?.isVideo == true && videoChromeState != null) {
                    VideoControlsBar(
                        isPlaying = videoChromeState.isPlaying,
                        isMuted = videoChromeState.isMuted,
                        durationMs = videoChromeState.durationMs,
                        positionMsProvider = videoChromeState.positionMsProvider,
                        bufferedMsProvider = videoChromeState.bufferedMsProvider,
                        activeQuality = videoChromeState.activeQuality,
                        qualityOptions = videoChromeState.qualityOptions,
                        showQualitySelector = videoChromeState.showQualitySelector,
                        onTogglePlayPause = videoChromeState.onTogglePlayPause,
                        onToggleMute = videoChromeState.onToggleMute,
                        onSeekTo = videoChromeState.onSeekTo,
                        onSelectQuality = videoChromeState.onSelectQuality,
                    )
                    Spacer(modifier = Modifier.height(10.dp))
                }

                Surface(
                    shape = CircleShape,
                    color = ViewerPillColor,
                    border = ViewerPillBorder,
                    // Consume taps until Add to / Bin are wired in Phases 4 & 5.
                    modifier = Modifier.clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                        onClick = {},
                    ),
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 7.dp, vertical = 5.dp),
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        ViewerPillAction(
                            imageVector = Icons.Default.Share,
                            label = if (isSharing) "Preparing…" else "Share",
                            enabled = isShareEnabled && !isSharing,
                            isLoading = isSharing,
                            onClick = onShare,
                        )
                        ViewerPillAction(imageVector = Icons.Default.Add, label = "Add to", enabled = false)
                        ViewerPillAction(imageVector = Icons.Outlined.Delete, label = "Bin", enabled = false)
                    }
                }
            }
        }
    }
}

@Composable
private fun ViewerStatusBarScrim(
    statusBarHeight: Dp,
    modifier: Modifier = Modifier,
) {
    if (statusBarHeight <= 0.dp) return
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(statusBarHeight + VIEWER_STATUS_BAR_SCRIM_TAIL)
            .background(
                Brush.verticalGradient(
                    0.0f to Color.Black.copy(alpha = 0.72f),
                    0.65f to Color.Black.copy(alpha = 0.35f),
                    1.0f to Color.Transparent,
                ),
            ),
    )
}

private val VIEWER_STATUS_BAR_SCRIM_TAIL = 14.dp

@Composable
private fun ViewerBottomControlsScrim(
    navBarHeight: Dp,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(navBarHeight + VIEWER_BOTTOM_SCRIM_HEIGHT)
            .background(
                Brush.verticalGradient(
                    0.0f to Color.Transparent,
                    0.38f to Color.Black.copy(alpha = 0.48f),
                    1.0f to Color.Black.copy(alpha = 0.78f),
                ),
            ),
    )
}

private val VIEWER_BOTTOM_SCRIM_HEIGHT = 190.dp

/** Dark translucent scrim with a hairline highlight, matching the web viewer's pills. */
private val ViewerPillColor = Color.Black.copy(alpha = 0.55f)
private val ViewerPillBorder = BorderStroke(1.dp, Color.White.copy(alpha = 0.12f))

@Composable
private fun ViewerCircleButton(
    imageVector: ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .size(40.dp)
            .clip(CircleShape)
            .background(ViewerPillColor)
            .border(1.dp, Color.White.copy(alpha = 0.12f), CircleShape)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = imageVector,
            contentDescription = contentDescription,
            tint = Color.White,
            modifier = Modifier.size(20.dp),
        )
    }
}

@Composable
private fun ViewerPillAction(
    imageVector: ImageVector,
    label: String,
    enabled: Boolean = true,
    isLoading: Boolean = false,
    onClick: () -> Unit = {},
) {
    val contentTint = if (enabled || isLoading) Color.White else Color.White.copy(alpha = 0.5f)
    Row(
        modifier = Modifier
            .clip(CircleShape)
            .then(
                if (enabled) {
                    Modifier.clickable(onClick = onClick)
                } else {
                    Modifier
                },
            )
            .padding(horizontal = 12.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                color = contentTint,
                strokeWidth = 2.dp,
                modifier = Modifier.size(18.dp),
            )
        } else {
            Icon(
                imageVector = imageVector,
                contentDescription = label,
                tint = contentTint,
                modifier = Modifier.size(21.dp),
            )
        }
        Spacer(modifier = Modifier.width(7.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = contentTint,
        )
    }
}
