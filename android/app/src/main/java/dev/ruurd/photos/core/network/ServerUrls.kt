package dev.ruurd.photos.core.network

import okhttp3.HttpUrl.Companion.toHttpUrlOrNull

fun normalizeServerUrl(url: String): String {
    val trimmed = url.trim().trimEnd('/')
    if (trimmed.isEmpty()) return ""
    return when {
        trimmed.startsWith("http://", ignoreCase = true) ->
            "http://" + trimmed.substring(7)

        trimmed.startsWith("https://", ignoreCase = true) ->
            "https://" + trimmed.substring(8)

        else -> "http://$trimmed"
    }
}

/**
 * True when the server URL points at a LAN/private host, where cleartext HTTP is expected and the
 * permission prompts differ from a public HTTPS server.
 */
fun isLikelyLocalServerUrl(rawUrl: String): Boolean {
    val normalized = normalizeServerUrl(rawUrl)
    if (normalized.isEmpty()) return false
    val host = normalized.toHttpUrlOrNull()?.host?.lowercase() ?: return false

    if (host == "localhost" || host == "127.0.0.1" || host == "::1") {
        return false
    }
    if (host.endsWith(".local") || host.endsWith(".lan") || host.endsWith(".home") || !host.contains('.')) {
        return true
    }
    val octets = host.split('.')
    if (octets.size == 4) {
        val first = octets[0].toIntOrNull() ?: return false
        val second = octets[1].toIntOrNull() ?: return false
        return when {
            first == 10 -> true
            first == 192 && second == 168 -> true
            first == 172 && second in 16..31 -> true
            first == 169 && second == 254 -> true
            else -> false
        }
    }
    return false
}
