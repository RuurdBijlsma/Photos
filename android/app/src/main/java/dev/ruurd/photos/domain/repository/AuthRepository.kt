package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.User
import kotlinx.coroutines.flow.StateFlow

interface AuthRepository {
    val authState: StateFlow<AuthState>
    val serverUrl: StateFlow<String>

    suspend fun setServerUrl(url: String)
    suspend fun testServerConnection(url: String): Result<Unit>
    suspend fun login(email: String, password: String): Result<User>
    suspend fun logout(): Result<Unit>
    suspend fun fetchCurrentUser(): Result<User>
}
