package dev.ruurd.photos.features.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.domain.repository.AuthRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class AuthUiState(
    val serverUrl: String = "",
    val email: String = "",
    val password: String = "",
    val isLoading: Boolean = false,
    val isTestingServer: Boolean = false,
    val serverTestSuccess: Boolean? = null,
    val errorMessage: String? = null
)

class AuthViewModel(
    private val authRepository: AuthRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(AuthUiState())
    val uiState: StateFlow<AuthUiState> = _uiState.asStateFlow()

    private var hasUserEditedServerUrl = false

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

        viewModelScope.launch {
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

    fun login() {
        val state = _uiState.value
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
                    _uiState.update { it.copy(isLoading = false) }
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

    companion object {
        fun factory(authRepository: AuthRepository): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T {
                    return AuthViewModel(authRepository) as T
                }
            }
    }
}