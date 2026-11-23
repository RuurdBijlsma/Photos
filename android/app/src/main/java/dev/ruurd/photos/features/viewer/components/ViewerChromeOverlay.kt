package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.collections.immutable.ImmutableList

/**
 * Floating viewer chrome (top bar, overflow menu, bottom action pill) plus the status bar scrim.
 * Anchored to the screen edges and faded as one layer during open, drag and close.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ViewerChromeOverlay(
    alpha: Float,
    items: ImmutableList<TimelineMediaProjection>,
    activeMediaId: String,
    currentPage: Int,
    isProjectionLoading: Boolean,
    isShareEnabled: Boolean,
    isCopyEnabled: Boolean,
    onBack: () -> Unit,
    onAbout: () -> Unit,
    onCopy: () -> Unit,
    onShare: () -> Unit,
    modifier: Modifier = Modifier,
) {
    // Ignoring visibility keeps the top bar anchored when statusBars() animates back in on tap.
    val statusBarPadding = WindowInsets.statusBarsIgnoringVisibility
        .asPaddingValues()
        .calculateTopPadding()
    val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

    Box(
        modifier = modifier
            .fillMaxSize()
            .graphicsLayer { this.alpha = alpha },
    ) {
        ViewerStatusBarScrim(
            statusBarHeight = statusBarPadding,
            modifier = Modifier.align(Alignment.TopCenter),
        )

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .align(Alignment.TopCenter)
                .padding(top = statusBarPadding + 6.dp, start = 8.dp, end = 8.dp),
        ) {
            ViewerCircleButton(
                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = "Back",
                onClick = onBack,
            )

            Spacer(modifier = Modifier.width(8.dp))

            if (!isProjectionLoading && items.isNotEmpty()) {
                Surface(
                    shape = CircleShape,
                    color = ViewerPillColor,
                    border = ViewerPillBorder,
                    modifier = Modifier.clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                        onClick = {},
                    ),
                ) {
                    val activeIndex = items.indexOfFirst { it.id == activeMediaId }
                    val displayIndex = ((if (activeIndex >= 0) activeIndex else currentPage) + 1)
                        .coerceIn(1, items.size)
                    Text(
                        text = "$displayIndex of ${items.size}",
                        style = MaterialTheme.typography.titleSmall.copy(
                            fontFamily = MontserratFontFamily,
                            fontWeight = FontWeight.SemiBold,
                        ),
                        color = Color.White,
                        modifier = Modifier.padding(horizontal = 14.dp, vertical = 9.dp),
                    )
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            var isMenuExpanded by remember { mutableStateOf(false) }
            Box {
                ViewerCircleButton(
                    imageVector = Icons.Default.MoreVert,
                    contentDescription = "Options",
                    onClick = { isMenuExpanded = true },
                )

                val currentItem = items.firstOrNull { it.id == activeMediaId }
                    ?: items.getOrNull(currentPage)
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
                    if (currentItem != null && !currentItem.isVideo && isCopyEnabled) {
                        DropdownMenuItem(
                            text = { Text("Copy") },
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
        }

        Surface(
            shape = CircleShape,
            color = ViewerPillColor,
            border = ViewerPillBorder,
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = navBarPadding + 14.dp)
                // Consume taps until Share / Add to / Bin are wired in Phases 2B, 4 & 5.
                .clickable(
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
                    label = "Share",
                    enabled = isShareEnabled,
                    onClick = onShare,
                )
                ViewerPillAction(imageVector = Icons.Default.Add, label = "Add to", enabled = false)
                ViewerPillAction(imageVector = Icons.Outlined.Delete, label = "Bin", enabled = false)
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

/** Dark translucent scrim with a hairline highlight, matching the web viewer's pills. */
private val ViewerPillColor = Color.Black.copy(alpha = 0.55f)
private val ViewerPillBorder = BorderStroke(1.dp, Color.White.copy(alpha = 0.12f))

@Composable
private fun ViewerCircleButton(
    imageVector: ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
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
    onClick: () -> Unit = {},
) {
    val contentTint = if (enabled) Color.White else Color.White.copy(alpha = 0.5f)
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
        Icon(
            imageVector = imageVector,
            contentDescription = label,
            tint = contentTint,
            modifier = Modifier.size(21.dp),
        )
        Spacer(modifier = Modifier.width(7.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = contentTint,
        )
    }
}
