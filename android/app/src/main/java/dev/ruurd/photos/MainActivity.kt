package dev.ruurd.photos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.features.auth.AuthScreen
import dev.ruurd.photos.features.auth.AuthViewModel
import dev.ruurd.photos.features.main.MainScreen
import dev.ruurd.photos.features.main.MainViewModel
import dev.ruurd.photos.features.timeline.TimelineViewModel
import dev.ruurd.photos.ui.theme.RuurdPhotosTheme

class MainActivity : ComponentActivity() {

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
                            MainScreen(
                                viewModel = mainViewModel,
                                timelineViewModel = timelineViewModel
                            )
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

    fun storeForUser(userId: Int): ViewModelStore {
        if (activeUserId != userId) {
            store?.clear()
            store = ViewModelStore()
            activeUserId = userId
        }
        return store!!
    }

    fun clearStore() {
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