package dev.ruurd.photos.features.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.repository.MediaRepository
import dev.ruurd.photos.domain.usecase.SyncTimelineUseCase
import dev.ruurd.photos.features.main.components.MainNavTab
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class MainUiState(
    val currentUser: User? = null,
    val serverUrl: String = "",
    val isOnline: Boolean = false,
    val syncState: SyncState = SyncState.IDLE,
    val visibleMediaCount: Int = 0,
    val totalMediaCount: Int = 0,
    val selectedTab: MainNavTab = MainNavTab.PHOTOS,
    val isAvatarMenuOpen: Boolean = false,
    val syncErrorMessage: String? = null
)

class MainViewModel(
    private val authRepository: AuthRepository,
    private val mediaRepository: MediaRepository,
    private val syncTimelineUseCase: SyncTimelineUseCase,
    networkMonitor: NetworkMonitor
) : ViewModel() {

    private val _selectedTab = MutableStateFlow(MainNavTab.PHOTOS)
    private val _isAvatarMenuOpen = MutableStateFlow(false)
    private val _syncErrorMessage = MutableStateFlow<String?>(null)

    private var hasRefreshedProfile = false

    private val connectionInfoFlow = combine(
        authRepository.serverUrl,
        networkMonitor.isOnline,
        syncTimelineUseCase.syncState
    ) { url, online, sync -> Triple(url, online, sync) }

    private val countsFlow = combine(
        mediaRepository.countVisible(),
        mediaRepository.countTotal()
    ) { visible, total -> visible to total }

    private val uiControlsFlow = combine(
        _selectedTab,
        _isAvatarMenuOpen,
        _syncErrorMessage
    ) { tab, menuOpen, error -> Triple(tab, menuOpen, error) }

    val uiState: StateFlow<MainUiState> = combine(
        authRepository.authState,
        connectionInfoFlow,
        countsFlow,
        uiControlsFlow
    ) { authState, conn, counts, controls ->
        val (serverUrl, isOnline, syncState) = conn
        val (visibleCount, totalCount) = counts
        val (tab, menuOpen, error) = controls
        val user = (authState as? AuthState.Authenticated)?.user

        MainUiState(
            currentUser = user,
            serverUrl = serverUrl,
            isOnline = isOnline,
            syncState = syncState,
            visibleMediaCount = visibleCount,
            totalMediaCount = totalCount,
            selectedTab = tab,
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
                        hasRefreshedProfile = false
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
        _selectedTab.value = tab
    }

    fun setAvatarMenuOpen(open: Boolean) {
        _isAvatarMenuOpen.value = open
    }

    fun syncNow(forceFullSnapshot: Boolean = false) {
        if (syncTimelineUseCase.syncState.value == SyncState.SYNCING) return

        viewModelScope.launch {
            _syncErrorMessage.value = null
            val result = syncTimelineUseCase(forceFullSnapshot = forceFullSnapshot)
            result.onFailure { error ->
                _syncErrorMessage.value = error.localizedMessage ?: "Sync failed"
            }
        }
    }

    fun logout() {
        viewModelScope.launch {
            authRepository.logout()
        }
    }

    companion object {
        fun factory(
            authRepository: AuthRepository,
            mediaRepository: MediaRepository,
            syncTimelineUseCase: SyncTimelineUseCase,
            networkMonitor: NetworkMonitor
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return MainViewModel(
                    authRepository = authRepository,
                    mediaRepository = mediaRepository,
                    syncTimelineUseCase = syncTimelineUseCase,
                    networkMonitor = networkMonitor
                ) as T
            }
        }
    }
}