package dev.ruurd.photos.core.network.auth

import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.cookie.PersistentCookieJar
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import okhttp3.Authenticator
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.Route

class AuthAuthenticator(
    private val cookieJar: PersistentCookieJar,
    private val sessionManager: SessionManager,
    private val scope: CoroutineScope
) : Authenticator {

    private val lock = Any()

    // Lightweight client without an Authenticator to prevent recursive 401 calls
    private val refreshClient by lazy {
        OkHttpClient.Builder()
            .cookieJar(cookieJar)
            .build()
    }

    override fun authenticate(route: Route?, response: Response): Request? {
        val requestPath = response.request.url.encodedPath

        // 1. Immediately abort if the failing request was login or refresh, or retried too many times
        if (requestPath.endsWith("/api/auth/login") ||
            requestPath.endsWith("/api/auth/refresh") ||
            responseCount(response) >= 2
        ) {
            return null
        }

        // Snapshot current access token from the request that failed
        val failingAccessToken = extractAccessTokenFromRequest(response.request)

        synchronized(lock) {
            // 2. Check if another concurrent thread already completed refresh while we were waiting
            val currentAccessToken = cookieJar.getCookie("access_token")?.value

            if (currentAccessToken != null && currentAccessToken != failingAccessToken) {
                // Token was already refreshed by another thread! Retry with new cookie
                return response.request.newBuilder().build()
            }

            // 3. Perform synchronized refresh call
            val baseUrl = sessionManager.getServerUrl().toHttpUrlOrNull() ?: return null
            val refreshUrl = baseUrl.newBuilder()
                .encodedPath("/api/auth/refresh")
                .build()

            val refreshRequest = Request.Builder()
                .url(refreshUrl)
                .post("".toRequestBody())
                .build()

            try {
                refreshClient.newCall(refreshRequest).execute().use { refreshResponse ->
                    if (refreshResponse.isSuccessful) {
                        // Refresh succeeded, new cookies are in CookieJar; retry original request
                        return response.request.newBuilder().build()
                    } else {
                        // Refresh token was invalid/expired: log user out
                        cookieJar.clear()
                        scope.launch {
                            sessionManager.setUnauthenticated()
                        }
                        return null
                    }
                }
            } catch (_: Exception) {
                return null
            }
        }
    }

    private fun extractAccessTokenFromRequest(request: Request): String? {
        val cookieHeader = request.header("Cookie") ?: return null
        return cookieHeader.split(";")
            .map { it.trim() }
            .firstOrNull { it.startsWith("access_token=") }
            ?.removePrefix("access_token=")
    }

    private fun responseCount(response: Response): Int {
        var count = 1
        var prior = response.priorResponse
        while (prior != null) {
            count++
            prior = prior.priorResponse
        }
        return count
    }
}
