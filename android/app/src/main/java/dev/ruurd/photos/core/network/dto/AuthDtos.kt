package dev.ruurd.photos.core.network.dto

data class LoginRequest(
    val email: String,
    val password: String
)

data class UserDto(
    val id: Int,
    val email: String,
    val name: String,
    val role: String,
    val mediaFolder: String? = null,
    val avatarId: String? = null
)
