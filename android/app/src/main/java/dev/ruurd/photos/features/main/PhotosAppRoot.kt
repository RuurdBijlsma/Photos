package dev.ruurd.photos.features.main

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.layout.LazyLayoutCacheWindow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.hideFromAccessibility
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntSize
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.LocalTimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import dev.ruurd.photos.core.navigation.AlbumDetailRoute
import dev.ruurd.photos.core.navigation.BackupSettingsRoute
import dev.ruurd.photos.core.navigation.DeviceFoldersRoute
import dev.ruurd.photos.core.navigation.MainRoute
import dev.ruurd.photos.di.AppContainer
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.features.auth.AuthScreen
import dev.ruurd.photos.features.auth.AuthViewModel
import dev.ruurd.photos.features.timeline.TimelineViewModel
import dev.ruurd.photos.features.viewer.MediaViewerScreen
import dev.ruurd.photos.features.viewer.MediaViewerViewModel
import kotlinx.coroutines.launch

/**
 * The app's composition root: auth gate, per-user ViewModel stores, the navigation shell and the
 * viewer overlay that sits above it. `MainActivity` only hosts this.
 */
@Composable
fun PhotosAppRoot(container: AppContainer) {
    val authState by container.authRepository.authState.collectAsState()
    val sessionHolder: SessionViewModelStoreHolder = viewModel(
        factory = SessionViewModelStoreHolder.FACTORY,
    )

    when (val state = authState) {
        is AuthState.Loading -> {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.background),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator()
            }
        }

        is AuthState.Unauthenticated -> {
            LaunchedEffect(Unit) {
                sessionHolder.clearStore()
            }
            val authViewModel: AuthViewModel = viewModel(
                factory = container.authViewModelFactory(),
            )
            AuthScreen(viewModel = authViewModel)
        }

        is AuthState.Authenticated -> AuthenticatedSession(
            container = container,
            sessionHolder = sessionHolder,
            user = state.user,
        )
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun AuthenticatedSession(
    container: AppContainer,
    sessionHolder: SessionViewModelStoreHolder,
    user: User,
) {
    val sessionOwner = remember(user.id, sessionHolder) {
        object : ViewModelStoreOwner {
            override val viewModelStore: ViewModelStore =
                sessionHolder.storeForUser(user.id)
        }
    }
    val boundsRegistry = remember(user.id) { TimelineTileBoundsRegistry() }

    CompositionLocalProvider(
        LocalViewModelStoreOwner provides sessionOwner,
        LocalMemoryCacheKeyRegistry provides container.memoryCacheKeyRegistry,
        LocalTimelineTileBoundsRegistry provides boundsRegistry,
    ) {
        val userKey = "user_${user.id}"
        val mainViewModel: MainViewModel = viewModel(
            key = "${userKey}_main",
            factory = container.mainViewModelFactory(),
        )
        val timelineViewModel: TimelineViewModel = viewModel(
            key = "${userKey}_timeline",
            factory = container.timelineViewModelFactory(),
        )

        val navController = rememberNavController()
        val timelineListState = rememberLazyListState(
            cacheWindow = remember {
                LazyLayoutCacheWindow(aheadFraction = 0.75f, behindFraction = 0.4f)
            },
        )
        val timelineUiState by timelineViewModel.uiState.collectAsState()
        val mainUiState by mainViewModel.uiState.collectAsState()
        val currentLayout by rememberUpdatedState(timelineUiState.layout)
        val coroutineScope = rememberCoroutineScope()

        var rootContainerSize by remember { mutableStateOf(IntSize.Zero) }
        var activeViewerMediaId by rememberSaveable(user.id) {
            mutableStateOf<String?>(null)
        }
        var viewerOpenSourceRect by remember { mutableStateOf<Rect?>(null) }
        var viewerOpenSourceCorners by remember { mutableStateOf(TileCornerRadii.Zero) }
        var hiddenTimelineMediaId by remember { mutableStateOf<String?>(null) }

        fun syncTimelineToMedia(mediaId: String) {
            val rowIndex = currentLayout.findRowIndexForMedia(mediaId) ?: return
            val layoutInfo = timelineListState.layoutInfo
            val clearTop = layoutInfo.viewportStartOffset + layoutInfo.beforeContentPadding
            val clearBottom = layoutInfo.viewportEndOffset - layoutInfo.afterContentPadding
            val itemInfo = layoutInfo.visibleItemsInfo.firstOrNull { it.index == rowIndex }
            val isFullyInClearViewport = itemInfo != null &&
                    itemInfo.offset >= clearTop &&
                    itemInfo.offset + itemInfo.size <= clearBottom
            if (!isFullyInClearViewport) {
                coroutineScope.launch {
                    timelineListState.scrollToItem(rowIndex)
                }
            }
        }

        Box(
            modifier = Modifier
                .fillMaxSize()
                .onSizeChanged { rootContainerSize = it },
        ) {
            // NavHost container (MainScreen root + pushed routes)
            // Suppress accessibility and focus while viewer overlay is open
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .then(
                        if (activeViewerMediaId != null) {
                            Modifier
                                .semantics { hideFromAccessibility() }
                                .focusProperties { canFocus = false }
                        } else {
                            Modifier
                        },
                    ),
            ) {
                NavHost(
                    navController = navController,
                    startDestination = MainRoute,
                    modifier = Modifier.fillMaxSize(),
                ) {
                    composable<MainRoute> {
                        MainScreen(
                            viewModel = mainViewModel,
                            timelineViewModel = timelineViewModel,
                            timelineListState = timelineListState,
                            hiddenTimelineMediaId = hiddenTimelineMediaId,
                            onMediaClick = { mediaId, sourceRect, sourceCorners ->
                                if (activeViewerMediaId == null) {
                                    viewerOpenSourceRect = sourceRect
                                    viewerOpenSourceCorners = sourceCorners
                                    activeViewerMediaId = mediaId
                                }
                            },
                        )
                    }
                    composable<DeviceFoldersRoute> {
                        // Pushed destination for Phase 3
                    }
                    composable<AlbumDetailRoute> {
                        // Pushed destination for Phase 5
                    }
                    composable<BackupSettingsRoute> {
                        // Pushed destination for Phase 3
                    }
                }
            }

            // MediaViewerScreen translucent overlay above NavHost
            val viewerMediaId = activeViewerMediaId
            if (viewerMediaId != null) {
                SessionMediaViewer(
                    container = container,
                    sessionHolder = sessionHolder,
                    userId = user.id,
                    userKey = userKey,
                    viewerMediaId = viewerMediaId,
                    baseUrl = mainUiState.serverUrl,
                    openSourceRect = viewerOpenSourceRect,
                    openSourceCorners = viewerOpenSourceCorners,
                    containerSize = rootContainerSize,
                    onHiddenTimelineMediaChanged = { hiddenTimelineMediaId = it },
                    onPreSyncScroll = ::syncTimelineToMedia,
                    onActiveMediaChanged = { activeId ->
                        if (activeViewerMediaId != null) {
                            activeViewerMediaId = activeId
                        }
                    },
                    onDismissFinished = {
                        hiddenTimelineMediaId = null
                        viewerOpenSourceRect = null
                        viewerOpenSourceCorners = TileCornerRadii.Zero
                        activeViewerMediaId = null
                        sessionHolder.clearViewerStore()
                    },
                )
            }
        }
    }
}

/** The viewer gets its own per-user store so reopening it always starts from the clicked item. */
@Composable
private fun SessionMediaViewer(
    container: AppContainer,
    sessionHolder: SessionViewModelStoreHolder,
    userId: Int,
    userKey: String,
    viewerMediaId: String,
    baseUrl: String,
    openSourceRect: Rect?,
    openSourceCorners: TileCornerRadii,
    containerSize: IntSize,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    onActiveMediaChanged: (String) -> Unit,
    onDismissFinished: () -> Unit,
) {
    val viewerOwner = remember(userId, sessionHolder) {
        object : ViewModelStoreOwner {
            override val viewModelStore: ViewModelStore =
                sessionHolder.viewerStoreForUser(userId)
        }
    }
    val viewerViewModel: MediaViewerViewModel = viewModel(
        viewModelStoreOwner = viewerOwner,
        key = "${userKey}_viewer",
        factory = container.mediaViewerViewModelFactory(viewerMediaId),
    )

    MediaViewerScreen(
        viewModel = viewerViewModel,
        baseUrl = baseUrl,
        openSourceRect = openSourceRect,
        openSourceCorners = openSourceCorners,
        containerSize = containerSize,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
        onPreSyncScroll = onPreSyncScroll,
        onDismissFinished = onDismissFinished,
        onActiveMediaChanged = onActiveMediaChanged,
    )
}

internal class SessionViewModelStoreHolder : ViewModel() {
    private var activeUserId: Int? = null
    private var store: ViewModelStore? = null
    private var viewerStore: ViewModelStore? = null

    fun storeForUser(userId: Int): ViewModelStore {
        if (activeUserId != userId) {
            viewerStore?.clear()
            viewerStore = null
            store?.clear()
            store = ViewModelStore()
            activeUserId = userId
        }
        return store!!
    }

    fun viewerStoreForUser(userId: Int): ViewModelStore {
        storeForUser(userId)
        if (viewerStore == null) {
            viewerStore = ViewModelStore()
        }
        return viewerStore!!
    }

    fun clearViewerStore() {
        viewerStore?.clear()
        viewerStore = null
    }

    fun clearStore() {
        viewerStore?.clear()
        viewerStore = null
        store?.clear()
        store = null
        activeUserId = null
    }

    override fun onCleared() {
        clearStore()
    }

    companion object {
        val FACTORY: ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T = SessionViewModelStoreHolder() as T
        }
    }
}