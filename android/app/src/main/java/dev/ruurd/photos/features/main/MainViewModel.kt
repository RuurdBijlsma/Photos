package dev.ruurd.photos.features.main

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.core.state.BootstrapState
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
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
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.io.IOException

enum class MainDestination {
    PHOTOS,
    COLLECTIONS,
    EXPLORE,
    SEARCH,
    ;

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
    val syncErrorMessage: String? = null,
)

class MainViewModel(
    private val authRepository: AuthRepository,
    private val syncTimelineUseCase: SyncTimelineUseCase,
    private val scanLocalMediaUseCase: ScanLocalMediaUseCase,
    private val localMediaStore: LocalMediaStore,
    private val bootstrapState: BootstrapState,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {

    private val destinationFlow = MutableStateFlow(MainDestination.PHOTOS)
    private var lastNonSearchTab: MainNavTab = MainNavTab.PHOTOS
    private val avatarMenuOpen = MutableStateFlow(false)
    private val syncErrorMessage = MutableStateFlow<String?>(null)

    private var hasRefreshedProfile = false
    private var isFirstLaunchScan = true
    private var syncJob: Job? = null
    private var scanJob: Job? = null
    private var pendingScanAccess: MediaAccessState? = null

    private val connectionInfoFlow = combine(
        authRepository.serverUrl,
        networkMonitor.isOnline,
        syncTimelineUseCase.syncState,
    ) { url, online, sync -> Triple(url, online, sync) }

    private val uiControlsFlow = combine(
        destinationFlow,
        avatarMenuOpen,
        syncErrorMessage,
    ) { destination, menuOpen, error -> Triple(destination, menuOpen, error) }

    val uiState: StateFlow<MainUiState> = combine(
        authRepository.authState,
        connectionInfoFlow,
        uiControlsFlow,
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
            syncErrorMessage = error,
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = MainUiState(),
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
                        isFirstLaunchScan = true
                        pendingScanAccess = null
                        syncJob?.cancel()
                        syncJob = null
                        scanJob?.cancel()
                        scanJob = null
                        lastNonSearchTab = MainNavTab.PHOTOS
                        destinationFlow.value = MainDestination.PHOTOS
                        avatarMenuOpen.value = false
                        syncErrorMessage.value = null
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

                        syncNow(forceFullSnapshot = false)
                    } else {
                        bootstrapState.onSyncSettled()
                    }
                }
        }
    }

    fun onTabSelected(tab: MainNavTab) {
        lastNonSearchTab = tab
        destinationFlow.value = when (tab) {
            MainNavTab.PHOTOS -> MainDestination.PHOTOS
            MainNavTab.COLLECTIONS -> MainDestination.COLLECTIONS
            MainNavTab.EXPLORE -> MainDestination.EXPLORE
        }
    }

    fun onSearchSelected() {
        destinationFlow.value = MainDestination.SEARCH
    }

    fun onBackFromSearch() {
        if (destinationFlow.value == MainDestination.SEARCH) {
            onTabSelected(lastNonSearchTab)
        }
    }

    fun setAvatarMenuOpen(open: Boolean) {
        avatarMenuOpen.value = open
    }

    fun clearSyncError() {
        syncErrorMessage.value = null
    }

    fun refreshTimeline(access: MediaAccessState) {
        if (access.hasAnyAccess) {
            scanLocalMedia(access, immediate = true)
        }
        if (networkMonitor.isOnline.value) {
            syncNow(forceFullSnapshot = true)
        }
    }

    fun syncNow(forceFullSnapshot: Boolean = false) {
        if (syncJob?.isActive == true || syncTimelineUseCase.syncState.value == SyncState.SYNCING) {
            if (!bootstrapState.isSyncSettled && syncJob?.isActive != true) {
                viewModelScope.launch {
                    syncTimelineUseCase.syncState.first { it != SyncState.SYNCING }
                    bootstrapState.onSyncSettled()
                }
            }
            return
        }

        syncJob = viewModelScope.launch {
            syncErrorMessage.value = null
            val result = syncTimelineUseCase(forceFullSnapshot = forceFullSnapshot)
            result.onFailure { error ->
                syncErrorMessage.value = formatSyncError(error)
            }
            bootstrapState.onSyncSettled()
        }
    }

    private fun formatSyncError(error: Throwable): String {
        val detail = error.localizedMessage?.takeIf { it.isNotBlank() }
        return when (error) {
            is IOException -> "Couldn't reach server to sync cloud library"
            else -> if (detail != null) "Cloud sync failed: $detail" else "Cloud sync failed"
        }
    }

    fun onMediaAccessDenied() {
        bootstrapState.onScanSettled()
    }

    suspend fun observeLocalMediaChanges(accessProvider: () -> MediaAccessState) {
        localMediaStore.observeChanges().collect {
            val currentAccess = accessProvider()
            if (currentAccess.hasAnyAccess) {
                scanLocalMedia(currentAccess)
            }
        }
    }

    fun scanLocalMedia(access: MediaAccessState, immediate: Boolean = false) {
        if (!access.hasAnyAccess) {
            bootstrapState.onScanSettled()
            return
        }
        if (scanJob?.isActive == true) {
            // Always queue a follow-up scan if a change/resume arrived mid-scan, even with the
            // same access state (e.g., IS_PENDING = 0 published while the current scan is running).
            pendingScanAccess = access
            return
        }

        val shouldDelayStartupScan =
            !immediate && isFirstLaunchScan && bootstrapState.isScanSettled
        isFirstLaunchScan = false

        lateinit var launchedJob: Job
        launchedJob = viewModelScope.launch {
            try {
                if (shouldDelayStartupScan) {
                    // Let the first timeline frame and its visible thumbnails land before competing for
                    // SQLite and MediaStore; the scan result only affects later frames.
                    delay(STARTUP_SCAN_DELAY_MS)
                }
                scanLocalMediaUseCase(access).onFailure { error ->
                    Log.w(TAG, "Local media scan failed", error)
                }
                bootstrapState.onScanSettled()
            } finally {
                val nextAccess = pendingScanAccess
                pendingScanAccess = null
                if (scanJob === launchedJob) {
                    scanJob = null
                }
                if (isActive &&
                    authRepository.authState.value is AuthState.Authenticated &&
                    nextAccess != null &&
                    nextAccess.hasAnyAccess
                ) {
                    scanLocalMedia(nextAccess, immediate = immediate)
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
    }
}
