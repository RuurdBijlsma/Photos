package dev.ruurd.photos.domain.model

data class DiscoveredServer(
    val url: String,
    val host: String,
    val port: Int,
    val latencyMs: Long,
    val isEmulatorHost: Boolean = false,
)
