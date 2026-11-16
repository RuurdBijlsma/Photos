package dev.ruurd.photos.features.main

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Collections
import androidx.compose.material.icons.filled.Explore
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import dev.ruurd.photos.core.permissions.MediaPermissions
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.features.main.components.AvatarMenuSheet
import dev.ruurd.photos.features.main.components.FloatingBottomNav
import dev.ruurd.photos.features.main.components.MainNavTab
import dev.ruurd.photos.features.main.components.PhotosTopBar
import dev.ruurd.photos.features.timeline.TimelineScreen
import dev.ruurd.photos.features.timeline.TimelineViewModel

@Composable
fun MainScreen(
    viewModel: MainViewModel,
    timelineViewModel: TimelineViewModel,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()

    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var mediaAccess: MediaAccessState by remember { mutableStateOf(MediaPermissions.read(context)) }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) {
        // ON_RESUME fires when the permission dialog closes and triggers the scan once.
        mediaAccess = MediaPermissions.read(context)
    }

    LaunchedEffect(Unit) {
        if (!mediaAccess.hasAnyAccess) {
            permissionLauncher.launch(MediaPermissions.requestedPermissions)
        }
    }

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                val currentAccess = MediaPermissions.read(context)
                mediaAccess = currentAccess
                if (currentAccess.hasAnyAccess) {
                    viewModel.scanLocalMedia(currentAccess)
                }
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

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
                    TimelineScreen(
                        viewModel = timelineViewModel,
                        baseUrl = uiState.serverUrl
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
            horizontalAlignment = Alignment.CenterHorizontally
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
