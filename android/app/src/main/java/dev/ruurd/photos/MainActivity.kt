package dev.ruurd.photos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.layout.LazyLayoutCacheWindow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.ExperimentalComposeUiApi
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
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import dev.ruurd.photos.core.navigation.AlbumDetailRoute
import dev.ruurd.photos.core.navigation.BackupSettingsRoute
import dev.ruurd.photos.core.navigation.DeviceFoldersRoute
import dev.ruurd.photos.core.navigation.MainRoute
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.features.auth.AuthScreen
import dev.ruurd.photos.features.auth.AuthViewModel
import dev.ruurd.photos.features.main.MainScreen
import dev.ruurd.photos.features.main.MainViewModel
import dev.ruurd.photos.features.timeline.TimelineViewModel
import dev.ruurd.photos.features.viewer.MediaViewerScreen
import dev.ruurd.photos.features.viewer.MediaViewerViewModel
import dev.ruurd.photos.ui.theme.RuurdPhotosTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    @OptIn(
        ExperimentalComposeUiApi::class,
        ExperimentalFoundationApi::class
    )
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val app = application as PhotosApplication
        val container = app.container

        setContent {
            RuurdPhotosTheme {
                val authState by container.authRepository.authState.collectAsState()
                val sessionHolder: SessionViewModelStoreHolder = viewModel(
                    factory = SessionViewModelStoreHolder.FACTORY
                )

                when (val state = authState) {
                    is AuthState.Loading -> {
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .background(MaterialTheme.colorScheme.background),
                            contentAlignment = Alignment.Center
                        ) {
                            CircularProgressIndicator()
                        }
                    }
                    is AuthState.Unauthenticated -> {
                        LaunchedEffect(Unit) {
                            sessionHolder.clearStore()
                        }
                        val authViewModel: AuthViewModel = viewModel(
                            factory = AuthViewModel.factory(container.authRepository)
                        )
                        AuthScreen(viewModel = authViewModel)
                    }
                    is AuthState.Authenticated -> {
                        val sessionOwner = remember(state.user.id, sessionHolder) {
                            object : ViewModelStoreOwner {
                                override val viewModelStore: ViewModelStore =
                                    sessionHolder.storeForUser(state.user.id)
                            }
                        }

                        CompositionLocalProvider(LocalViewModelStoreOwner provides sessionOwner) {
                            val userKey = "user_${state.user.id}"
                            val mainViewModel: MainViewModel = viewModel(
                                key = "${userKey}_main",
                                factory = MainViewModel.factory(
                                    authRepository = container.authRepository,
                                    syncTimelineUseCase = container.syncTimelineUseCase,
                                    scanLocalMediaUseCase = container.scanLocalMediaUseCase,
                                    localMediaStore = container.localMediaStore,
                                    syncTimestampStore = container.sessionManager,
                                    scanStateStore = container.sessionManager,
                                    networkMonitor = container.networkMonitor
                                )
                            )
                            val timelineViewModel: TimelineViewModel = viewModel(
                                key = "${userKey}_timeline",
                                factory = TimelineViewModel.factory(
                                    observeTimelineUseCase = container.observeTimelineUseCase,
                                    calculateTimelineLayoutUseCase = container.calculateTimelineLayoutUseCase,
                                    dispatchers = container.dispatchers,
                                    syncTimestampStore = container.sessionManager,
                                    scanStateStore = container.sessionManager
                                )
                            )

                            val navController = rememberNavController()
                            val timelineListState = rememberLazyListState(
                                cacheWindow = remember {
                                    LazyLayoutCacheWindow(aheadFraction = 0.75f, behindFraction = 0.4f)
                                }
                            )
                            val timelineUiState by timelineViewModel.uiState.collectAsState()
                            val mainUiState by mainViewModel.uiState.collectAsState()
                            val currentLayout by rememberUpdatedState(timelineUiState.layout)
                            val coroutineScope = rememberCoroutineScope()

                            val boundsRegistry = remember(state.user.id) { TimelineTileBoundsRegistry() }
                            var rootContainerSize by remember { mutableStateOf(IntSize.Zero) }

                            var activeViewerMediaId by rememberSaveable(state.user.id) {
                                mutableStateOf<String?>(null)
                            }
                            var viewerOpenSourceRect by remember { mutableStateOf<Rect?>(null) }
                            var hiddenTimelineMediaId by remember { mutableStateOf<String?>(null) }

                            fun syncTimelineToMedia(mediaId: String) {
                                val rowIndex = currentLayout.findRowIndexForMedia(mediaId) ?: return
                                val isVisible = timelineListState.layoutInfo.visibleItemsInfo
                                    .any { it.index == rowIndex }
                                if (!isVisible) {
                                    coroutineScope.launch {
                                        timelineListState.scrollToItem(rowIndex)
                                    }
                                }
                            }

                            Box(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .onSizeChanged { rootContainerSize = it }
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
                                            }
                                        )
                                ) {
                                    NavHost(
                                        navController = navController,
                                        startDestination = MainRoute,
                                        modifier = Modifier.fillMaxSize()
                                    ) {
                                        composable<MainRoute> {
                                            MainScreen(
                                                viewModel = mainViewModel,
                                                timelineViewModel = timelineViewModel,
                                                timelineListState = timelineListState,
                                                memoryCacheKeyRegistry = container.memoryCacheKeyRegistry,
                                                boundsRegistry = boundsRegistry,
                                                hiddenTimelineMediaId = hiddenTimelineMediaId,
                                                onMediaClick = { mediaId, sourceRect ->
                                                    if (activeViewerMediaId == null) {
                                                        viewerOpenSourceRect = sourceRect
                                                        activeViewerMediaId = mediaId
                                                    }
                                                }
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
                                    val viewerOwner = remember(state.user.id, sessionHolder) {
                                        object : ViewModelStoreOwner {
                                            override val viewModelStore: ViewModelStore =
                                                sessionHolder.viewerStoreForUser(state.user.id)
                                        }
                                    }
                                    val viewerViewModel: MediaViewerViewModel = viewModel(
                                        viewModelStoreOwner = viewerOwner,
                                        key = "${userKey}_viewer",
                                        factory = MediaViewerViewModel.factory(
                                            initialMediaId = viewerMediaId,
                                            timelineProjectionStore = container.timelineProjectionStore,
                                            mediaRepository = container.mediaRepository,
                                            dispatchers = container.dispatchers
                                        )
                                    )

                                    MediaViewerScreen(
                                        viewModel = viewerViewModel,
                                        baseUrl = mainUiState.serverUrl,
                                        imageLoader = container.imageLoader,
                                        memoryCacheKeyRegistry = container.memoryCacheKeyRegistry,
                                        boundsRegistry = boundsRegistry,
                                        openSourceRect = viewerOpenSourceRect,
                                        containerSize = rootContainerSize,
                                        onHiddenTimelineMediaChanged = { hiddenTimelineMediaId = it },
                                        onPreSyncScroll = ::syncTimelineToMedia,
                                        onDismissFinished = {
                                            hiddenTimelineMediaId = null
                                            viewerOpenSourceRect = null
                                            activeViewerMediaId = null
                                            sessionHolder.clearViewerStore()
                                        }
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
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
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return SessionViewModelStoreHolder() as T
            }
        }
    }
}