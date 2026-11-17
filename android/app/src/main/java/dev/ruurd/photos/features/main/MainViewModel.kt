package dev.ruurd.photos.features.main

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.usecase.ScanLocalMediaUseCase
import dev.ruurd.photos.domain.usecase.SyncTimelineUseCase
import dev.ruurd.photos.features.main.components.MainNavTab
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import java.io.IOException

enum class MainDestination {
    PHOTOS,
    COLLECTIONS,
    EXPLORE,
    SEARCH;

    fun asNavTabOrNull(): MainNavTab? = when (this) {
        PHOTOS -> MainNavTab.PHOTOS
        COLLECTIONS -> MainNavTab.COLLECTIONS
        EXPLORE -> MainNavTab.EXPLORE
        SEARCH -> null
    }
}

data class MainUiState(
    val currentUser: User? = null,
    val serverUrl: String = "",
    val isOnline: Boolean = false,
    val syncState: SyncState = SyncState.IDLE,
    val destination: MainDestination = MainDestination.PHOTOS,
    val isAvatarMenuOpen: Boolean = false,
    val syncErrorMessage: String? = null
)

class MainViewModel(
    private val authRepository: AuthRepository,
    private val syncTimelineUseCase: SyncTimelineUseCase,
    private val scanLocalMediaUseCase: ScanLocalMediaUseCase,
    networkMonitor: NetworkMonitor
) : ViewModel() {

    private val _destination = MutableStateFlow(MainDestination.PHOTOS)
    private var lastNonSearchTab: MainNavTab = MainNavTab.PHOTOS
    private val _isAvatarMenuOpen = MutableStateFlow(false)
    private val _syncErrorMessage = MutableStateFlow<String?>(null)

    private var hasRefreshedProfile = false
    private var scanJob: Job? = null
    private var pendingScanAccess: MediaAccessState? = null

    private val connectionInfoFlow = combine(
        authRepository.serverUrl,
        networkMonitor.isOnline,
        syncTimelineUseCase.syncState
    ) { url, online, sync -> Triple(url, online, sync) }

    private val uiControlsFlow = combine(
        _destination,
        _isAvatarMenuOpen,
        _syncErrorMessage
    ) { destination, menuOpen, error -> Triple(destination, menuOpen, error) }

    val uiState: StateFlow<MainUiState> = combine(
        authRepository.authState,
        connectionInfoFlow,
        uiControlsFlow
    ) { authState, conn, controls ->
        val (serverUrl, isOnline, syncState) = conn
        val (destination, menuOpen, error) = controls
        val user = (authState as? AuthState.Authenticated)?.user

        MainUiState(
            currentUser = user,
            serverUrl = serverUrl,
            isOnline = isOnline,
            syncState = syncState,
            destination = destination,
            isAvatarMenuOpen = menuOpen,
            syncErrorMessage = error
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = MainUiState()
    )

    init {
        // Trigger background profile refresh and delta sync on launch and when connectivity returns
        viewModelScope.launch {
            combine(authRepository.authState, networkMonitor.isOnline) { auth, online ->
                (auth is AuthState.Authenticated) to online
            }
                .distinctUntilChanged()
                .collect { (isAuthenticated, isOnline) ->
                    if (!isAuthenticated) {
                        // Reset per-session UI so a later login does not inherit stale state.
                        hasRefreshedProfile = false
                        pendingScanAccess = null
                        scanJob?.cancel()
                        scanJob = null
                        lastNonSearchTab = MainNavTab.PHOTOS
                        _destination.value = MainDestination.PHOTOS
                        _isAvatarMenuOpen.value = false
                        _syncErrorMessage.value = null
                        return@collect
                    }

                    if (isOnline) {
                        if (!hasRefreshedProfile) {
                            viewModelScope.launch {
                                if (authRepository.fetchCurrentUser().isSuccess) {
                                    hasRefreshedProfile = true
                                }
                            }
                        }

                        if (syncTimelineUseCase.syncState.value != SyncState.SYNCING) {
                            syncNow(forceFullSnapshot = false)
                        }
                    }
                }
        }
    }

    fun onTabSelected(tab: MainNavTab) {
        lastNonSearchTab = tab
        _destination.value = when (tab) {
            MainNavTab.PHOTOS -> MainDestination.PHOTOS
            MainNavTab.COLLECTIONS -> MainDestination.COLLECTIONS
            MainNavTab.EXPLORE -> MainDestination.EXPLORE
        }
    }

    fun onSearchSelected() {
        _destination.value = MainDestination.SEARCH
    }

    fun onBackFromSearch() {
        if (_destination.value == MainDestination.SEARCH) {
            onTabSelected(lastNonSearchTab)
        }
    }

    fun setAvatarMenuOpen(open: Boolean) {
        _isAvatarMenuOpen.value = open
    }

    fun clearSyncError() {
        _syncErrorMessage.value = null
    }

    fun syncNow(forceFullSnapshot: Boolean = false) {
        if (syncTimelineUseCase.syncState.value == SyncState.SYNCING) return

        viewModelScope.launch {
            _syncErrorMessage.value = null
            val result = syncTimelineUseCase(forceFullSnapshot = forceFullSnapshot)
            result.onFailure { error ->
                _syncErrorMessage.value = formatSyncError(error)
            }
        }
    }

    private fun formatSyncError(error: Throwable): String {
        val detail = error.localizedMessage?.takeIf { it.isNotBlank() }
        return when (error) {
            is IOException -> "Couldn't reach server to sync cloud library"
            else -> if (detail != null) "Cloud sync failed: $detail" else "Cloud sync failed"
        }
    }

    fun scanLocalMedia(access: MediaAccessState) {
        if (!access.hasAnyAccess) return
        if (scanJob?.isActive == true) {
            pendingScanAccess = access
            return
        }

        lateinit var launchedJob: Job
        launchedJob = viewModelScope.launch {
            try {
                // Let the first timeline frame and its visible thumbnails land before competing for
                // SQLite and MediaStore; the scan result only affects later frames.
                delay(STARTUP_SCAN_DELAY_MS)
                scanLocalMediaUseCase(access).onFailure { error ->
                    Log.w(TAG, "Local media scan failed", error)
                }
            } finally {
                val nextAccess = pendingScanAccess
                pendingScanAccess = null
                if (scanJob === launchedJob) {
                    scanJob = null
                }
                if (nextAccess != null && nextAccess.hasAnyAccess) {
                    scanLocalMedia(nextAccess)
                }
            }
        }
        scanJob = launchedJob
    }

    fun logout() {
        viewModelScope.launch {
            authRepository.logout()
        }
    }

    companion object {
        private const val TAG = "MainViewModel"
        private const val STARTUP_SCAN_DELAY_MS = 500L

        fun factory(
            authRepository: AuthRepository,
            syncTimelineUseCase: SyncTimelineUseCase,
            scanLocalMediaUseCase: ScanLocalMediaUseCase,
            networkMonitor: NetworkMonitor
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return MainViewModel(
                    authRepository = authRepository,
                    syncTimelineUseCase = syncTimelineUseCase,
                    scanLocalMediaUseCase = scanLocalMediaUseCase,
                    networkMonitor = networkMonitor
                ) as T
            }
        }
    }
}