package dev.ruurd.photos.features.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.discovery.LanScanEvent
import dev.ruurd.photos.domain.model.DiscoveredServer
import dev.ruurd.photos.domain.repository.AuthRepository
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull

data class AuthUiState(
    val serverUrl: String = "",
    val email: String = "",
    val password: String = "",
    val isLoading: Boolean = false,
    val isTestingServer: Boolean = false,
    val serverTestSuccess: Boolean? = null,
    val isScanningLan: Boolean = false,
    val scanProgress: Float? = null,
    val hasScannedLan: Boolean = false,
    val discoveredServers: List<DiscoveredServer> = emptyList(),
    val errorMessage: String? = null
)

class AuthViewModel(
    private val authRepository: AuthRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(AuthUiState())
    val uiState: StateFlow<AuthUiState> = _uiState.asStateFlow()

    private var hasUserEditedServerUrl = false
    private var serverTestJob: Job? = null
    private var lanScanJob: Job? = null

    init {
        viewModelScope.launch {
            authRepository.serverUrl.collect { url ->
                if (!hasUserEditedServerUrl) {
                    _uiState.update { it.copy(serverUrl = url) }
                }
            }
        }
    }

    fun onServerUrlChanged(url: String) {
        hasUserEditedServerUrl = true
        _uiState.update {
            it.copy(
                serverUrl = url,
                serverTestSuccess = null,
                errorMessage = null
            )
        }
    }

    /** Rewrites only the scheme, so tapping an `http`/`https` chip never eats the host or port. */
    fun onServerSchemeSelected(scheme: String) {
        val current = _uiState.value.serverUrl
        onServerUrlChanged("$scheme://${current.substringAfter("://", current).trimStart('/')}")
    }

    fun cancelServerTest() {
        serverTestJob?.cancel()
        serverTestJob = null
        _uiState.update {
            it.copy(isTestingServer = false, serverTestSuccess = null, errorMessage = null)
        }
    }

    fun scanLocalNetwork() {
        val currentState = _uiState.value
        if (currentState.isScanningLan || currentState.isLoading) return

        cancelServerTest()
        val prioritizedHost = SessionManager.normalizeUrl(currentState.serverUrl)
            .toHttpUrlOrNull()
            ?.host

        lanScanJob = viewModelScope.launch {
            _uiState.update {
                it.copy(
                    isScanningLan = true,
                    scanProgress = 0f,
                    hasScannedLan = false,
                    discoveredServers = emptyList(),
                    errorMessage = null
                )
            }

            try {
                authRepository.scanLocalServers(prioritizedHost).collect { event ->
                    when (event) {
                        is LanScanEvent.Found -> {
                            _uiState.update { state ->
                                val merged = (state.discoveredServers + event.server)
                                    .distinctBy { it.url }
                                    .sortedWith(
                                        compareByDescending<DiscoveredServer> { it.isEmulatorHost }
                                            .thenBy { it.latencyMs }
                                    )
                                state.copy(discoveredServers = merged)
                            }
                        }
                        is LanScanEvent.Progress -> {
                            val fraction = event.completed.toFloat() / event.total.coerceAtLeast(1)
                            _uiState.update { it.copy(scanProgress = fraction.coerceIn(0f, 1f)) }
                        }
                        LanScanEvent.NoLocalNetwork -> {
                            _uiState.update {
                                it.copy(
                                    isScanningLan = false,
                                    scanProgress = null,
                                    hasScannedLan = false,
                                    errorMessage = "Connect to Wi-Fi to scan your local network."
                                )
                            }
                        }
                    }
                }
            } finally {
                _uiState.update { state ->
                    if (state.isScanningLan) {
                        state.copy(
                            isScanningLan = false,
                            scanProgress = null,
                            hasScannedLan = true
                        )
                    } else {
                        state
                    }
                }
                lanScanJob = null
            }
        }
    }

    fun cancelLanScan() {
        lanScanJob?.cancel()
        lanScanJob = null
        _uiState.update {
            it.copy(isScanningLan = false, scanProgress = null)
        }
    }

    fun selectDiscoveredServer(server: DiscoveredServer) {
        hasUserEditedServerUrl = true
        _uiState.update {
            it.copy(
                serverUrl = server.url,
                serverTestSuccess = true,
                errorMessage = null
            )
        }
        viewModelScope.launch {
            authRepository.setServerUrl(server.url)
        }
    }

    fun onEmailChanged(email: String) {
        _uiState.update {
            it.copy(
                email = email,
                errorMessage = null
            )
        }
    }

    fun onPasswordChanged(password: String) {
        _uiState.update {
            it.copy(
                password = password,
                errorMessage = null
            )
        }
    }

    fun testServerConnection() {
        val currentState = _uiState.value
        if (currentState.isTestingServer || currentState.isLoading) return

        val normalizedUrl = SessionManager.normalizeUrl(currentState.serverUrl)
        if (normalizedUrl.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "Server URL cannot be empty") }
            return
        }

        serverTestJob = viewModelScope.launch {
            _uiState.update {
                it.copy(
                    isTestingServer = true,
                    serverTestSuccess = null,
                    errorMessage = null
                )
            }

            val result = authRepository.testServerConnection(normalizedUrl)
            result.fold(
                onSuccess = {
                    authRepository.setServerUrl(normalizedUrl)
                    _uiState.update {
                        it.copy(
                            serverUrl = normalizedUrl,
                            isTestingServer = false,
                            serverTestSuccess = true,
                            errorMessage = null
                        )
                    }
                },
                onFailure = { error ->
                    _uiState.update {
                        it.copy(
                            isTestingServer = false,
                            serverTestSuccess = false,
                            errorMessage = "Cannot reach server: ${error.localizedMessage ?: "Unknown error"}"
                        )
                    }
                }
            )
        }
    }

    fun login() {        val state = _uiState.value
        if (state.isLoading) return

        val normalizedUrl = SessionManager.normalizeUrl(state.serverUrl)
        val email = state.email.trim()
        val password = state.password

        if (normalizedUrl.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "Please enter server URL") }
            return
        }
        if (email.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "Please enter email") }
            return
        }
        if (password.isEmpty()) {
            _uiState.update { it.copy(errorMessage = "Please enter password") }
            return
        }

        cancelLanScan()

        viewModelScope.launch {
            _uiState.update {
                it.copy(
                    serverUrl = normalizedUrl,
                    isLoading = true,
                    errorMessage = null
                )
            }
            authRepository.setServerUrl(normalizedUrl)

            val loginResult = authRepository.login(email, password)
            loginResult.fold(
                onSuccess = {
                    _uiState.update { it.copy(isLoading = false, password = "") }
                },
                onFailure = { error ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            errorMessage = error.localizedMessage ?: "Login failed"
                        )
                    }
                }
            )
        }
    }
}