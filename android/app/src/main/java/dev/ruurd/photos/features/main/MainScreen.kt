package dev.ruurd.photos.features.main

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Collections
import androidx.compose.material.icons.filled.Explore
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
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
import dev.ruurd.photos.features.main.components.FloatingBottomNavHeight
import dev.ruurd.photos.features.main.components.MainNavTab
import dev.ruurd.photos.features.main.components.PhotosTopBar
import dev.ruurd.photos.features.main.components.PhotosTopBarHeight
import dev.ruurd.photos.features.timeline.TimelineScreen
import dev.ruurd.photos.features.timeline.TimelineViewModel

@Composable
fun MainScreen(
    viewModel: MainViewModel,
    timelineViewModel: TimelineViewModel,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val timelineListState = rememberLazyListState()

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

    // The grid runs edge to edge behind both floating bars, so the insets are content padding only.
    val topInset = WindowInsets.statusBars.asPaddingValues().calculateTopPadding() +
            PhotosTopBarHeight
    val bottomInset = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding() +
            FloatingBottomNavHeight

    // Hide on scroll-down, show again after scrolling up the same distance (not only at the very top).
    var topBarHidden by remember { mutableStateOf(false) }
    LaunchedEffect(timelineListState) {
        var lastIndex = timelineListState.firstVisibleItemIndex
        var lastOffset = timelineListState.firstVisibleItemScrollOffset
        var travelled = 0f
        snapshotFlow {
            timelineListState.firstVisibleItemIndex to timelineListState.firstVisibleItemScrollOffset
        }.collect { (index, offset) ->
            // An index change also resets the offset, so weight it far above any real offset delta.
            val delta = (index - lastIndex) * INDEX_WEIGHT_PX + (offset - lastOffset)
            lastIndex = index
            lastOffset = offset
            if (delta == 0f) return@collect
            if (travelled * delta < 0f) travelled = 0f
            travelled += delta
            if (travelled > TOP_BAR_HIDE_THRESHOLD_PX) topBarHidden = true
            if (travelled < -TOP_BAR_HIDE_THRESHOLD_PX) topBarHidden = false
        }
    }
    val hideTopBar = uiState.selectedTab == MainNavTab.PHOTOS && topBarHidden

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        when (uiState.selectedTab) {
            MainNavTab.PHOTOS -> {
                TimelineScreen(
                    viewModel = timelineViewModel,
                    baseUrl = uiState.serverUrl,
                    listState = timelineListState,
                    contentPadding = PaddingValues(top = topInset, bottom = bottomInset)
                )
            }
            MainNavTab.COLLECTIONS -> {
                PlaceholderTabContent(
                    title = "Collections",
                    subtitle = "Albums, Device Folders and People will appear here in Phase 2 & 3.",
                    icon = Icons.Default.Collections,
                    modifier = Modifier.padding(top = topInset, bottom = bottomInset)
                )
            }
            MainNavTab.EXPLORE -> {
                PlaceholderTabContent(
                    title = "Explore",
                    subtitle = "Search by places, categories, and memories.",
                    icon = Icons.Default.Explore,
                    modifier = Modifier.padding(top = topInset, bottom = bottomInset)
                )
            }
        }

        AnimatedVisibility(
            visible = !hideTopBar,
            enter = slideInVertically { -it } + fadeIn(),
            exit = slideOutVertically { -it } + fadeOut(),
            modifier = Modifier.align(Alignment.TopCenter)
        ) {
            PhotosTopBar(
                currentUser = uiState.currentUser,
                isOnline = uiState.isOnline,
                syncState = uiState.syncState,
                onAvatarClick = { viewModel.setAvatarMenuOpen(true) },
                modifier = Modifier.background(MaterialTheme.colorScheme.background)
            )
        }

        FloatingBottomNav(
            selectedTab = uiState.selectedTab,
            onTabSelected = viewModel::onTabSelected,
            onSearchClick = { /* Handled in future phases */ },
            modifier = Modifier.align(Alignment.BottomCenter)
        )

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

private const val TOP_BAR_HIDE_THRESHOLD_PX = 96f
private const val INDEX_WEIGHT_PX = 100_000f
