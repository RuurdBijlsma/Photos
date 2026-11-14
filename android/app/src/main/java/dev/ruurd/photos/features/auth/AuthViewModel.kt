package dev.ruurd.photos.features.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.domain.repository.AuthRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
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

    init {
        viewModelScope.launch {
            authRepository.serverUrl.collect { url ->
                if (_uiState.value.serverUrl.isEmpty()) {
                    _uiState.value = _uiState.value.copy(serverUrl = url)
                }
            }
        }
    }

    fun onServerUrlChanged(url: String) {
        _uiState.value = _uiState.value.copy(
            serverUrl = url,
            serverTestSuccess = null,
            errorMessage = null
        )
    }

    fun onEmailChanged(email: String) {
        _uiState.value = _uiState.value.copy(
            email = email,
            errorMessage = null
        )
    }

    fun onPasswordChanged(password: String) {
        _uiState.value = _uiState.value.copy(
            password = password,
            errorMessage = null
        )
    }

    fun testServerConnection() {
        val url = _uiState.value.serverUrl.trim()
        if (url.isEmpty()) {
            _uiState.value = _uiState.value.copy(errorMessage = "Server URL cannot be empty")
            return
        }

        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isTestingServer = true, errorMessage = null)
            val result = authRepository.testServerConnection(url)
            result.fold(
                onSuccess = {
                    authRepository.setServerUrl(url)
                    _uiState.value = _uiState.value.copy(
                        isTestingServer = false,
                        serverTestSuccess = true,
                        errorMessage = null
                    )
                },
                onFailure = { error ->
                    _uiState.value = _uiState.value.copy(
                        isTestingServer = false,
                        serverTestSuccess = false,
                        errorMessage = "Cannot reach server: ${error.localizedMessage}"
                    )
                }
            )
        }
    }

    fun login() {
        val state = _uiState.value
        val url = state.serverUrl.trim()
        val email = state.email.trim()
        val password = state.password

        if (url.isEmpty()) {
            _uiState.value = state.copy(errorMessage = "Please enter server URL")
            return
        }
        if (email.isEmpty()) {
            _uiState.value = state.copy(errorMessage = "Please enter email")
            return
        }
        if (password.isEmpty()) {
            _uiState.value = state.copy(errorMessage = "Please enter password")
            return
        }

        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isLoading = true, errorMessage = null)
            authRepository.setServerUrl(url)

            val loginResult = authRepository.login(email, password)
            loginResult.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isLoading = false)
                },
                onFailure = { error ->
                    _uiState.value = _uiState.value.copy(
                        isLoading = false,
                        errorMessage = error.localizedMessage ?: "Login failed"
                    )
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
