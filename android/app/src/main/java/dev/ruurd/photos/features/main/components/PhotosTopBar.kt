package dev.ruurd.photos.features.main.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.withInfiniteAnimationFrameNanos
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.R
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.ui.theme.LocalSemanticColors
import kotlinx.coroutines.isActive
import kotlin.math.ceil

/** Height of the top bar content, excluding the status bar inset. */
val PhotosTopBarHeight = 56.dp

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun PhotosTopBar(
    currentUser: User?,
    isOnline: Boolean,
    syncState: SyncState,
    onAvatarClick: () -> Unit,
    modifier: Modifier = Modifier,
    hasSyncError: Boolean = false,
    pullToRefreshState: PullToRefreshState? = null,
    onStatusPillClick: () -> Unit = {},
) {
    val semanticColors = LocalSemanticColors.current
    val density = LocalDensity.current

    val isSyncing = syncState == SyncState.SYNCING
    val isPulling = pullToRefreshState?.isPulling == true
    val isArmed = pullToRefreshState?.isArmed == true
    val offlineFeedbackCount = pullToRefreshState?.offlineFeedbackCount ?: 0

    // Spin the flower-petal mark smoothly while syncing and coast upright when done.
    val syncSpinAngle = remember { Animatable(0f) }
    LaunchedEffect(isSyncing) {
        if (isSyncing) {
            var lastFrameNanos = withInfiniteAnimationFrameNanos { it }
            while (isActive) {
                val frameNanos = withInfiniteAnimationFrameNanos { it }
                val dtSeconds = ((frameNanos - lastFrameNanos) / 1_000_000_000f)
                    .coerceIn(0f, 0.05f)
                lastFrameNanos = frameNanos
                syncSpinAngle.snapTo(syncSpinAngle.value + dtSeconds * 220f)
            }
        } else if (syncSpinAngle.value != 0f) {
            val target = ceil(syncSpinAngle.value / 360f) * 360f
            syncSpinAngle.animateTo(
                targetValue = target,
                animationSpec = spring(
                    dampingRatio = 0.85f,
                    stiffness = Spring.StiffnessLow,
                ),
            )
            syncSpinAngle.snapTo(0f)
        }
    }

    val logoScale by animateFloatAsState(
        targetValue = if (isPulling && isArmed && !isSyncing) 1.12f else 1f,
        animationSpec = spring(dampingRatio = 0.5f, stiffness = Spring.StiffnessMedium),
        label = "logo_pull_scale",
    )

    val pillScale by animateFloatAsState(
        targetValue = if (isPulling && isArmed && !isSyncing) 1.06f else 1f,
        animationSpec = spring(dampingRatio = 0.55f, stiffness = Spring.StiffnessMedium),
        label = "pill_pull_scale",
    )

    val offlineShakeOffsetDp = remember { Animatable(0f) }
    LaunchedEffect(offlineFeedbackCount) {
        if (offlineFeedbackCount > 0) {
            val keyframes = listOf(-8f, 8f, -6f, 5f, -3f, 0f)
            for (offset in keyframes) {
                offlineShakeOffsetDp.animateTo(
                    targetValue = offset,
                    animationSpec = tween(durationMillis = 48),
                )
            }
        }
    }

    Box(
        modifier = modifier
            .fillMaxWidth()
            .windowInsetsPadding(WindowInsets.statusBarsIgnoringVisibility)
            .height(PhotosTopBarHeight)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        // Left: Ruurd Photos flower-petal logo mark (rotates with pull and during sync)
        Image(
            painter = painterResource(id = R.drawable.icon_transparent),
            contentDescription = "Ruurd Photos",
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .align(Alignment.CenterStart)
                .size(32.dp)
                .graphicsLayer {
                    val pullProgress = pullToRefreshState?.pullProgress ?: 0f
                    rotationZ = syncSpinAngle.value + pullProgress * 220f
                    scaleX = logoScale
                    scaleY = logoScale
                },
        )

        // Center: Connectivity / Sync / Pull-to-refresh Status Pill
        val showStatusPill = !isOnline ||
                isPulling ||
                isSyncing ||
                syncState == SyncState.SERVER_UNREACHABLE ||
                syncState == SyncState.ERROR ||
                hasSyncError

        AnimatedVisibility(
            visible = showStatusPill,
            enter = fadeIn() + scaleIn(initialScale = 0.88f),
            exit = fadeOut() + scaleOut(targetScale = 0.88f),
            modifier = Modifier.align(Alignment.Center),
        ) {
            val (pillText, pillColor, pillIcon) = when {
                isSyncing -> Triple("Syncing...", semanticColors.syncing, Icons.Default.Sync)

                !isOnline -> Triple(
                    if (isPulling && isArmed) "No connection" else "Offline",
                    semanticColors.offline,
                    Icons.Default.CloudOff,
                )

                isPulling -> Triple(
                    if (isArmed) "Release to sync" else "Refresh",
                    semanticColors.syncing,
                    Icons.Default.Sync,
                )

                syncState == SyncState.SERVER_UNREACHABLE ->
                    Triple("Server unreachable", semanticColors.error, Icons.Default.CloudOff)

                syncState == SyncState.ERROR || hasSyncError ->
                    Triple("Sync failed", semanticColors.error, Icons.Default.ErrorOutline)

                else -> Triple("", Color.Transparent, Icons.Default.Sync)
            }

            val showPullFill = isPulling && !isSyncing
            val fillForeground = if (pillColor.luminance() > 0.45f) {
                Color(0xFF0B1320)
            } else {
                Color.White
            }

            Box(
                modifier = Modifier
                    .graphicsLayer {
                        scaleX = pillScale
                        scaleY = pillScale
                        translationX = offlineShakeOffsetDp.value * density.density
                    }
                    .height(30.dp)
                    .clip(CircleShape)
                    .background(
                        color = pillColor.copy(alpha = 0.15f),
                        shape = CircleShape,
                    )
                    .then(
                        if (showPullFill) {
                            Modifier.border(
                                width = 1.dp,
                                color = pillColor.copy(alpha = 0.32f),
                                shape = CircleShape,
                            )
                        } else {
                            Modifier
                        },
                    )
                    .clickable(
                        enabled = isOnline && !isSyncing && !isPulling,
                        onClick = onStatusPillClick,
                    )
                    .animateContentSize(
                        animationSpec = spring(
                            dampingRatio = Spring.DampingRatioNoBouncy,
                            stiffness = Spring.StiffnessMedium,
                        ),
                    ),
                contentAlignment = Alignment.Center,
            ) {
                StatusPillContent(
                    text = pillText,
                    icon = pillIcon,
                    isSyncing = isSyncing,
                    rotateIconWithPull = showPullFill && isOnline,
                    pullToRefreshState = pullToRefreshState,
                    contentColor = pillColor,
                    modifier = Modifier.padding(horizontal = 12.dp),
                )

                if (showPullFill) {
                    Box(
                        modifier = Modifier
                            .matchParentSize()
                            .drawWithContent {
                                val fraction = (pullToRefreshState?.pullProgress ?: 0f)
                                    .coerceIn(0f, 1f)
                                val fillWidth = size.width * fraction
                                if (fillWidth > 0f) {
                                    clipRect(right = fillWidth) {
                                        this@drawWithContent.drawContent()
                                    }
                                }
                            }
                            .background(pillColor),
                        contentAlignment = Alignment.Center,
                    ) {
                        StatusPillContent(
                            text = pillText,
                            icon = pillIcon,
                            isSyncing = false,
                            rotateIconWithPull = isOnline,
                            pullToRefreshState = pullToRefreshState,
                            contentColor = fillForeground,
                            modifier = Modifier.padding(horizontal = 12.dp),
                        )
                    }
                }
            }
        }

        // Right: Circular User Avatar
        val initial = currentUser?.name?.take(1)?.uppercase()
            ?: currentUser?.email?.take(1)?.uppercase()
            ?: "U"

        Box(
            modifier = Modifier
                .align(Alignment.CenterEnd)
                .size(38.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primaryContainer)
                .clickable(onClick = onAvatarClick),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = initial,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
                style = MaterialTheme.typography.titleMedium.copy(
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp,
                ),
            )
        }
    }
}

@Composable
private fun StatusPillContent(
    text: String,
    icon: ImageVector,
    isSyncing: Boolean,
    rotateIconWithPull: Boolean,
    pullToRefreshState: PullToRefreshState?,
    contentColor: Color,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
    ) {
        if (isSyncing) {
            CircularProgressIndicator(
                color = contentColor,
                strokeWidth = 2.dp,
                modifier = Modifier.size(14.dp),
            )
        } else {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = contentColor,
                modifier = Modifier
                    .size(14.dp)
                    .graphicsLayer {
                        if (rotateIconWithPull) {
                            rotationZ = (pullToRefreshState?.pullProgress ?: 0f) * 220f
                        }
                    },
            )
        }
        Spacer(modifier = Modifier.width(6.dp))
        Text(
            text = text,
            color = contentColor,
            maxLines = 1,
            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
        )
    }
}