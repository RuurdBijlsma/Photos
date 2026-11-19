package dev.ruurd.photos.core.state

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged

/**
 * First-launch gate shared by the session ViewModels: the timeline stays empty until the initial
 * sync and the initial media scan have both reported, whichever order they finish in.
 */
class BootstrapState(needsBootstrap: Boolean) {
    private val syncSettled = MutableStateFlow(!needsBootstrap)
    private val scanSettled = MutableStateFlow(!needsBootstrap)

    val isBootstrapping: Flow<Boolean> = combine(syncSettled, scanSettled) { sync, scan ->
        !sync || !scan
    }.distinctUntilChanged()

    val isSyncSettled: Boolean get() = syncSettled.value
    val isScanSettled: Boolean get() = scanSettled.value

    fun onSyncSettled() {
        syncSettled.value = true
    }

    fun onScanSettled() {
        scanSettled.value = true
    }

    /** A new session starts with cleared sync/scan state and has to bootstrap again. */
    fun reset() {
        syncSettled.value = false
        scanSettled.value = false
    }
}
