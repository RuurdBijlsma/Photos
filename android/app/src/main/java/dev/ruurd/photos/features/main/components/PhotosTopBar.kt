package dev.ruurd.photos.features.main.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.R
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.ui.theme.LocalSemanticColors

/** Height of the top bar content, excluding the status bar inset. */
val PhotosTopBarHeight = 56.dp

@Composable
fun PhotosTopBar(
    currentUser: User?,
    isOnline: Boolean,
    syncState: SyncState,
    hasSyncError: Boolean = false,
    onStatusPillClick: () -> Unit = {},
    onAvatarClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val semanticColors = LocalSemanticColors.current

    Box(
        modifier = modifier
            .fillMaxWidth()
            .statusBarsPadding()
            .height(PhotosTopBarHeight)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center
    ) {
        // Left: Ruurd Photos flower-petal logo mark
        Image(
            painter = painterResource(id = R.drawable.icon_transparent),
            contentDescription = "Ruurd Photos",
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .align(Alignment.CenterStart)
                .size(32.dp)
        )

        // Center: Subtle Connectivity / Sync Status Pill (hidden when idle & online with no error)
        val showStatusPill = !isOnline ||
                syncState == SyncState.SYNCING ||
                syncState == SyncState.SERVER_UNREACHABLE ||
                syncState == SyncState.ERROR ||
                hasSyncError

        AnimatedVisibility(
            visible = showStatusPill,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier.align(Alignment.Center)
        ) {
            val (pillText, pillColor, pillIcon) = when {
                !isOnline -> Triple("Offline", semanticColors.offline, Icons.Default.CloudOff)
                syncState == SyncState.SYNCING -> Triple("Syncing...", semanticColors.syncing, Icons.Default.Sync)
                syncState == SyncState.SERVER_UNREACHABLE -> Triple("Server unreachable", semanticColors.error, Icons.Default.CloudOff)
                syncState == SyncState.ERROR || hasSyncError -> Triple("Sync failed", semanticColors.error, Icons.Default.ErrorOutline)
                else -> Triple("", Color.Transparent, Icons.Default.Sync)
            }

            Box(
                modifier = Modifier
                    .height(30.dp)
                    .clip(CircleShape)
                    .background(
                        color = pillColor.copy(alpha = 0.15f),
                        shape = CircleShape
                    )
                    .clickable(enabled = isOnline && syncState != SyncState.SYNCING, onClick = onStatusPillClick)
                    .padding(horizontal = 12.dp),
                contentAlignment = Alignment.Center
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center
                ) {
                    if (syncState == SyncState.SYNCING) {
                        CircularProgressIndicator(
                            color = pillColor,
                            strokeWidth = 2.dp,
                            modifier = Modifier.size(14.dp)
                        )
                    } else {
                        Icon(
                            imageVector = pillIcon,
                            contentDescription = null,
                            tint = pillColor,
                            modifier = Modifier.size(14.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = pillText,
                        color = pillColor,
                        style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold)
                    )
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
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = initial,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
                style = MaterialTheme.typography.titleMedium.copy(
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
            )
        }
    }
}