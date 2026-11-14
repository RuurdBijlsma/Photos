package dev.ruurd.photos.features.main

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudDone
import androidx.compose.material.icons.filled.Collections
import androidx.compose.material.icons.filled.Explore
import androidx.compose.material.icons.filled.PhotoLibrary
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.features.main.components.AvatarMenuSheet
import dev.ruurd.photos.features.main.components.FloatingBottomNav
import dev.ruurd.photos.features.main.components.MainNavTab
import dev.ruurd.photos.features.main.components.PhotosTopBar
import dev.ruurd.photos.ui.theme.PhotosRadii

@Composable
fun MainScreen(
    viewModel: MainViewModel,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()

    Scaffold(
        modifier = modifier.fillMaxSize(),
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            PhotosTopBar(
                currentUser = uiState.currentUser,
                isOnline = uiState.isOnline,
                syncState = uiState.syncState,
                onAvatarClick = { viewModel.setAvatarMenuOpen(true) }
            )
        },
        bottomBar = {
            FloatingBottomNav(
                selectedTab = uiState.selectedTab,
                onTabSelected = viewModel::onTabSelected,
                onSearchClick = { /* Handled in future phases */ }
            )
        }
    ) { contentPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(contentPadding)
        ) {
            when (uiState.selectedTab) {
                MainNavTab.PHOTOS -> {
                    PhotosTabContent(
                        uiState = uiState,
                        onSyncClick = { viewModel.syncNow(forceFullSnapshot = false) }
                    )
                }
                MainNavTab.COLLECTIONS -> {
                    PlaceholderTabContent(
                        title = "Collections",
                        subtitle = "Albums, Device Folders and People will appear here in Phase 2 & 3.",
                        icon = Icons.Default.Collections
                    )
                }
                MainNavTab.EXPLORE -> {
                    PlaceholderTabContent(
                        title = "Explore",
                        subtitle = "Search by places, categories, and memories.",
                        icon = Icons.Default.Explore
                    )
                }
            }

            // Avatar Menu Bottom Sheet
            if (uiState.isAvatarMenuOpen) {
                AvatarMenuSheet(
                    currentUser = uiState.currentUser,
                    serverUrl = uiState.serverUrl,
                    isOnline = uiState.isOnline,
                    syncState = uiState.syncState,
                    onSyncClick = { viewModel.syncNow(forceFullSnapshot = true) },
                    onLogoutClick = viewModel::logout,
                    onDismissRequest = { viewModel.setAvatarMenuOpen(false) }
                )
            }
        }
    }
}

@Composable
private fun PhotosTabContent(
    uiState: MainUiState,
    onSyncClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 20.dp, vertical = 16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // Status & Sync Overview Card
        Card(
            shape = RoundedCornerShape(PhotosRadii.Card),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surfaceContainerHigh
            ),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text(
                            text = "Cloud Sync & Storage",
                            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = if (uiState.isOnline) "Connected to server" else "Offline mode (Room SSOT active)",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }

                    if (uiState.syncState == SyncState.SYNCING) {
                        CircularProgressIndicator(
                            strokeWidth = 2.dp,
                            modifier = Modifier.size(24.dp)
                        )
                    } else {
                        Icon(
                            imageVector = Icons.Default.CloudDone,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(24.dp)
                        )
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    MetricBox(
                        title = "In Timeline",
                        value = "${uiState.visibleMediaCount}",
                        modifier = Modifier.weight(1f)
                    )
                    MetricBox(
                        title = "Total Synced",
                        value = "${uiState.totalMediaCount}",
                        modifier = Modifier.weight(1f)
                    )
                }

                AnimatedVisibility(visible = uiState.syncErrorMessage != null) {
                    uiState.syncErrorMessage?.let { error ->
                        Text(
                            text = error,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall
                        )
                    }
                }

                Button(
                    onClick = onSyncClick,
                    enabled = uiState.isOnline && uiState.syncState != SyncState.SYNCING,
                    shape = CircleShape,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(44.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Sync,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = if (uiState.syncState == SyncState.SYNCING) "Syncing..." else "Sync Cloud Library",
                        style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold)
                    )
                }
            }
        }

        // Ready container for Phase 1B (Local MediaStore) & Phase 1C (Justified Timeline)
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f)
                .background(
                    color = MaterialTheme.colorScheme.surfaceContainerLow,
                    shape = RoundedCornerShape(PhotosRadii.Card)
                )
                .padding(24.dp),
            contentAlignment = Alignment.Center
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                Icon(
                    imageVector = Icons.Default.PhotoLibrary,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.6f),
                    modifier = Modifier.size(48.dp)
                )
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = if (uiState.visibleMediaCount > 0) {
                        "Cloud library synchronized (${uiState.visibleMediaCount} items)\nReady for Phase 1B/1C justified grid"
                    } else {
                        "Tap 'Sync Cloud Library' above to fetch items from your server"
                    },
                    style = MaterialTheme.typography.bodyMedium.copy(
                        textAlign = TextAlign.Center,
                        lineHeight = 22.sp
                    ),
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
private fun MetricBox(
    title: String,
    value: String,
    modifier: Modifier = Modifier
) {
    Card(
        shape = RoundedCornerShape(PhotosRadii.NestedCard),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainer
        ),
        modifier = modifier
    ) {
        Column(
            modifier = Modifier.padding(12.dp)
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.Bold),
                color = MaterialTheme.colorScheme.onSurface
            )
        }
    }
}

@Composable
private fun PlaceholderTabContent(
    title: String,
    subtitle: String,
    icon: ImageVector,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .padding(32.dp),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f),
                modifier = Modifier.size(56.dp)
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = title,
                style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.Bold),
                color = MaterialTheme.colorScheme.onBackground
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodyMedium.copy(textAlign = TextAlign.Center),
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
