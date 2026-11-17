package dev.ruurd.photos.features.main

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.lazy.layout.LazyLayoutCacheWindow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Collections
import androidx.compose.material.icons.filled.Explore
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import dev.ruurd.photos.core.permissions.MediaPermissions
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.features.main.components.AvatarMenuSheet
import dev.ruurd.photos.features.main.components.FloatingBottomNav
import dev.ruurd.photos.features.main.components.FloatingBottomNavHeight
import dev.ruurd.photos.features.main.components.MainNavTab
import dev.ruurd.photos.features.main.components.PhotosTopBar
import dev.ruurd.photos.features.main.components.PhotosTopBarHeight
import dev.ruurd.photos.features.timeline.TimelineScreen
import dev.ruurd.photos.features.timeline.TimelineViewModel
import kotlinx.coroutines.launch

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun MainScreen(
    viewModel: MainViewModel,
    timelineViewModel: TimelineViewModel,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val timelineCacheWindow = remember {
        LazyLayoutCacheWindow(aheadFraction = 0.75f, behindFraction = 0.4f)
    }
    val timelineListState = rememberLazyListState(cacheWindow = timelineCacheWindow)
    val coroutineScope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var mediaAccess: MediaAccessState by remember { mutableStateOf(MediaPermissions.read(context)) }
    var hasRequestedPermissions by rememberSaveable { mutableStateOf(false) }
    var isPermissionDialogPending by rememberSaveable { mutableStateOf(false) }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) {
        isPermissionDialogPending = false
        val currentAccess = MediaPermissions.read(context)
        mediaAccess = currentAccess
        if (currentAccess.hasAnyAccess) {
            viewModel.scanLocalMedia(currentAccess)
        } else {
            viewModel.onMediaAccessDenied()
        }
    }

    BackHandler(enabled = uiState.destination == MainDestination.SEARCH) {
        viewModel.onBackFromSearch()
    }

    LaunchedEffect(Unit) {
        if (!mediaAccess.hasAnyAccess && !hasRequestedPermissions) {
            hasRequestedPermissions = true
            isPermissionDialogPending = true
            permissionLauncher.launch(MediaPermissions.requestedPermissions)
        }
    }

    // Observe MediaStore changes while foregrounded (STARTED -> STOPPED) so late IS_PENDING = 0
    // reveals after returning from the camera appear automatically.
    LaunchedEffect(lifecycleOwner, mediaAccess.hasAnyAccess) {
        if (!mediaAccess.hasAnyAccess) return@LaunchedEffect
        lifecycleOwner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            viewModel.observeLocalMediaChanges { MediaPermissions.read(context) }
        }
    }

    LaunchedEffect(uiState.syncErrorMessage) {
        val message = uiState.syncErrorMessage ?: return@LaunchedEffect
        val result = snackbarHostState.showSnackbar(
            message = message,
            actionLabel = "Retry",
            withDismissAction = true,
            duration = SnackbarDuration.Long
        )
        when (result) {
            SnackbarResult.ActionPerformed -> viewModel.syncNow()
            SnackbarResult.Dismissed -> viewModel.clearSyncError()
        }
    }

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                val currentAccess = MediaPermissions.read(context)
                mediaAccess = currentAccess
                if (currentAccess.hasAnyAccess) {
                    viewModel.scanLocalMedia(currentAccess)
                } else if (hasRequestedPermissions && !isPermissionDialogPending) {
                    viewModel.onMediaAccessDenied()
                }
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    // The grid runs edge to edge behind both floating bars, so the insets are content padding only.
    val statusBarHeight = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val topInset = statusBarHeight + PhotosTopBarHeight
    val bottomInset = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding() +
            FloatingBottomNavHeight

    // Hide on scroll-down, reveal on scroll-up or near the top; ignore non-scroll layout shifts.
    var topBarHidden by remember { mutableStateOf(false) }
    LaunchedEffect(timelineListState) {
        var lastIndex = timelineListState.firstVisibleItemIndex
        var lastOffset = timelineListState.firstVisibleItemScrollOffset
        var travelled = 0f
        snapshotFlow {
            Triple(
                timelineListState.isScrollInProgress,
                timelineListState.firstVisibleItemIndex,
                timelineListState.firstVisibleItemScrollOffset
            )
        }.collect { (isScrolling, index, offset) ->
            if (index == 0 && offset <= TOP_BAR_HIDE_THRESHOLD_PX.toInt()) {
                topBarHidden = false
                if (offset == 0) {
                    travelled = 0f
                    lastIndex = 0
                    lastOffset = 0
                    return@collect
                }
            }
            if (!isScrolling) {
                lastIndex = index
                lastOffset = offset
                travelled = 0f
                return@collect
            }
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
    val hideTopBar = uiState.destination == MainDestination.PHOTOS && topBarHidden

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        when (uiState.destination) {
            MainDestination.PHOTOS -> {
                TimelineScreen(
                    viewModel = timelineViewModel,
                    baseUrl = uiState.serverUrl,
                    listState = timelineListState,
                    contentPadding = PaddingValues(top = topInset, bottom = bottomInset),
                    topBarHeight = PhotosTopBarHeight,
                    topBarVisible = !hideTopBar,
                    isSyncing = uiState.syncState == SyncState.SYNCING,
                    isBootstrapping = uiState.isBootstrapping
                )
            }
            MainDestination.COLLECTIONS -> {
                PlaceholderTabContent(
                    title = "Collections",
                    subtitle = "Albums, Device Folders and People will appear here in Phase 2 & 3.",
                    icon = Icons.Default.Collections,
                    modifier = Modifier.padding(top = topInset, bottom = bottomInset)
                )
            }
            MainDestination.EXPLORE -> {
                PlaceholderTabContent(
                    title = "Explore",
                    subtitle = "Explore your stats, locations, and memories.",
                    icon = Icons.Default.Explore,
                    modifier = Modifier.padding(top = topInset, bottom = bottomInset)
                )
            }
            MainDestination.SEARCH -> {
                PlaceholderTabContent(
                    title = "Search",
                    subtitle = "Semantic, people, and place search will appear here.",
                    icon = Icons.Default.Search,
                    modifier = Modifier.padding(top = topInset, bottom = bottomInset)
                )
            }
        }

        // Kept behind PhotosTopBar (and shorter than it) so it stays hidden while the bar is
        // visible and seamlessly keeps status bar icons legible when the bar scrolls away.
        if (uiState.destination == MainDestination.PHOTOS) {
            StatusBarScrim(
                statusBarHeight = statusBarHeight,
                modifier = Modifier.align(Alignment.TopCenter)
            )
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
                hasSyncError = uiState.syncErrorMessage != null,
                onStatusPillClick = {
                    if (uiState.isOnline && uiState.syncState != SyncState.SYNCING) {
                        viewModel.syncNow()
                    }
                },
                onAvatarClick = { viewModel.setAvatarMenuOpen(true) },
                modifier = Modifier.background(MaterialTheme.colorScheme.background)
            )
        }

        SnackbarHost(
            hostState = snackbarHostState,
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .navigationBarsPadding()
                .padding(bottom = FloatingBottomNavHeight + 4.dp, start = 16.dp, end = 16.dp)
        )

        FloatingBottomNav(
            selectedTab = uiState.destination.asNavTabOrNull(),
            showSearchButton = uiState.destination != MainDestination.SEARCH,
            onTabSelected = { tab ->
                if (tab == MainNavTab.PHOTOS && uiState.destination == MainDestination.PHOTOS) {
                    topBarHidden = false
                    coroutineScope.launch {
                        timelineListState.animateScrollToItem(0)
                    }
                } else {
                    viewModel.onTabSelected(tab)
                }
            },
            onSearchClick = viewModel::onSearchSelected,
            modifier = Modifier.align(Alignment.BottomCenter)
        )

        // Avatar Menu Bottom Sheet
        if (uiState.isAvatarMenuOpen) {
            AvatarMenuSheet(
                currentUser = uiState.currentUser,
                serverUrl = uiState.serverUrl,
                isOnline = uiState.isOnline,
                syncState = uiState.syncState,
                syncErrorMessage = uiState.syncErrorMessage,
                onSyncClick = { viewModel.syncNow(forceFullSnapshot = true) },
                onLogoutClick = viewModel::logout,
                onDismissRequest = { viewModel.setAvatarMenuOpen(false) }
            )
        }
    }
}

@Composable
private fun StatusBarScrim(
    statusBarHeight: Dp,
    modifier: Modifier = Modifier
) {
    if (statusBarHeight <= 0.dp) return
    val scrimColor = MaterialTheme.colorScheme.background
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(statusBarHeight + STATUS_BAR_SCRIM_TAIL)
            .background(
                Brush.verticalGradient(
                    0.0f to scrimColor.copy(alpha = 0.95f),
                    0.65f to scrimColor.copy(alpha = 0.55f),
                    1.0f to Color.Transparent
                )
            )
    )
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

private val STATUS_BAR_SCRIM_TAIL = 24.dp
private const val TOP_BAR_HIDE_THRESHOLD_PX = 96f
private const val INDEX_WEIGHT_PX = 100_000f