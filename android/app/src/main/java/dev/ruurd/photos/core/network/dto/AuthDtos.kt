package dev.ruurd.photos.core.network.dto

import kotlinx.serialization.Serializable

@Serializable
data class LoginRequest(
    val email: String,
    val password: String
)

@Serializable
data class UserDto(
    val id: Int,
    val email: String,
    val name: String,
    val role: String = "user",
    val mediaFolder: String? = null,
    val avatarId: String? = null
)
