package dev.ruurd.photos.features.main

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
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

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

/**
 * The main shell's navigation, avatar menu and session chrome. Library sync and the local media scan
 * live in [LibrarySyncCoordinator], which this exposes to the screen.
 */
class MainViewModel(
    private val authRepository: AuthRepository,
    private val syncTimelineUseCase: SyncTimelineUseCase,
    scanLocalMediaUseCase: ScanLocalMediaUseCase,
    localMediaStore: LocalMediaStore,
    bootstrapState: BootstrapState,
    networkMonitor: NetworkMonitor,
) : ViewModel() {

    private val destinationFlow = MutableStateFlow(MainDestination.PHOTOS)
    private var lastNonSearchTab: MainNavTab = MainNavTab.PHOTOS
    private val avatarMenuOpen = MutableStateFlow(false)

    private val librarySync = LibrarySyncCoordinator(
        scope = viewModelScope,
        authRepository = authRepository,
        syncTimelineUseCase = syncTimelineUseCase,
        scanLocalMediaUseCase = scanLocalMediaUseCase,
        localMediaStore = localMediaStore,
        bootstrapState = bootstrapState,
        networkMonitor = networkMonitor,
        onSignedOut = ::resetForSignOut,
    )

    private val connectionInfoFlow = combine(
        authRepository.serverUrl,
        networkMonitor.isOnline,
        syncTimelineUseCase.syncState,
    ) { url, online, sync -> Triple(url, online, sync) }

    private val uiControlsFlow = combine(
        destinationFlow,
        avatarMenuOpen,
        librarySync.syncErrorMessage,
    ) { destination, menuOpen, error -> Triple(destination, menuOpen, error) }

    val uiState: StateFlow<MainUiState> = combine(
        authRepository.authState,
        connectionInfoFlow,
        uiControlsFlow,
    ) { authState, conn, controls ->
        val (serverUrl, isOnline, syncState) = conn
        val (destination, menuOpen, error) = controls

        MainUiState(
            currentUser = (authState as? AuthState.Authenticated)?.user,
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
        librarySync.start()
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

    private fun resetForSignOut() {
        lastNonSearchTab = MainNavTab.PHOTOS
        destinationFlow.value = MainDestination.PHOTOS
        avatarMenuOpen.value = false
    }

    fun clearSyncError() = librarySync.clearSyncError()

    fun refreshTimeline(access: MediaAccessState) = librarySync.refreshTimeline(access)

    fun syncNow(forceFullSnapshot: Boolean = false) = librarySync.syncNow(forceFullSnapshot = forceFullSnapshot)

    fun onMediaAccessDenied() = librarySync.onMediaAccessDenied()

    suspend fun observeLocalMediaChanges(accessProvider: () -> MediaAccessState) = librarySync.observeLocalMediaChanges(accessProvider)

    fun scanLocalMedia(access: MediaAccessState, immediate: Boolean = false) = librarySync.scanLocalMedia(access, immediate = immediate)

    fun logout() {
        viewModelScope.launch {
            authRepository.logout()
        }
    }
}
