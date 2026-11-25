package dev.ruurd.photos.features.main

import android.util.Log
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.core.state.BootstrapState
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
import dev.ruurd.photos.domain.usecase.ScanLocalMediaUseCase
import dev.ruurd.photos.domain.usecase.SyncTimelineUseCase
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.io.IOException

/**
 * Runs the library's two background pipelines for the signed-in session: the server delta sync and
 * the local MediaStore scan, including the bootstrap milestones they report.
 *
 * Separate from [MainViewModel] so the screen's state stays navigation and chrome while the sync
 * lifecycle (profile refresh, first-launch delay, queued follow-up scans, per-session reset) has one
 * owner. Call [start] once with a scope tied to the signed-in session.
 */
class LibrarySyncCoordinator(
    private val scope: CoroutineScope,
    private val authRepository: AuthRepository,
    private val syncTimelineUseCase: SyncTimelineUseCase,
    private val scanLocalMediaUseCase: ScanLocalMediaUseCase,
    private val localMediaStore: LocalMediaStore,
    private val bootstrapState: BootstrapState,
    private val networkMonitor: NetworkMonitor,
    private val onSignedOut: () -> Unit,
) {
    private val _syncErrorMessage = MutableStateFlow<String?>(null)
    val syncErrorMessage: StateFlow<String?> = _syncErrorMessage.asStateFlow()

    private var hasRefreshedProfile = false
    private var isFirstLaunchScan = true
    private var syncJob: Job? = null
    private var scanJob: Job? = null
    private var pendingScanAccess: MediaAccessState? = null

    /** Delta-syncs on launch and whenever connectivity returns. */
    fun start() {
        scope.launch {
            combine(authRepository.authState, networkMonitor.isOnline) { auth, online ->
                (auth is AuthState.Authenticated) to online
            }
                .distinctUntilChanged()
                .collect { (isAuthenticated, isOnline) ->
                    if (!isAuthenticated) {
                        resetForSignOut()
                        onSignedOut()
                        return@collect
                    }

                    if (isOnline) {
                        if (!hasRefreshedProfile) {
                            scope.launch {
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

    private fun resetForSignOut() {
        // Reset per-session state so a later login does not inherit it.
        hasRefreshedProfile = false
        isFirstLaunchScan = true
        pendingScanAccess = null
        syncJob?.cancel()
        syncJob = null
        scanJob?.cancel()
        scanJob = null
        _syncErrorMessage.value = null
    }

    fun clearSyncError() {
        _syncErrorMessage.value = null
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
                scope.launch {
                    syncTimelineUseCase.syncState.first { it != SyncState.SYNCING }
                    bootstrapState.onSyncSettled()
                }
            }
            return
        }

        syncJob = scope.launch {
            _syncErrorMessage.value = null
            val result = syncTimelineUseCase(forceFullSnapshot = forceFullSnapshot)
            result.onFailure { error ->
                _syncErrorMessage.value = formatSyncError(error)
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
        launchedJob = scope.launch {
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

    private companion object {
        const val TAG = "LibrarySyncCoordinator"
        const val STARTUP_SCAN_DELAY_MS = 500L
    }
}
