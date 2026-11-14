package dev.ruurd.photos.domain.model

data class User(
    val id: Int,
    val email: String,
    val name: String,
    val role: String,
    val mediaFolder: String? = null,
    val avatarId: String? = null
)

sealed interface AuthState {
    data object Loading : AuthState
    data object Unauthenticated : AuthState
    data class Authenticated(val user: User) : AuthState
}

enum class SyncState {
    IDLE,
    SYNCING,
    OFFLINE,
    SERVER_UNREACHABLE
}
