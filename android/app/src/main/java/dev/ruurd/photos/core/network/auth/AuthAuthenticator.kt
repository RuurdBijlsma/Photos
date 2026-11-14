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
import java.util.concurrent.TimeUnit

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
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(15, TimeUnit.SECONDS)
            .writeTimeout(15, TimeUnit.SECONDS)
            .build()
    }

    override fun authenticate(route: Route?, response: Response): Request? {
        val requestPath = response.request.url.encodedPath

        // 1. Abort if the failing request was login/refresh, or already retried
        if (requestPath.endsWith("/api/auth/login") ||
            requestPath.endsWith("/api/auth/refresh") ||
            responseCount(response) >= 2
        ) {
            return null
        }

        // Snapshot access token from the request that failed
        val failingAccessToken = extractAccessTokenFromRequest(response.request)

        synchronized(lock) {
            // 2. Check if another thread already refreshed the token while we waited
            val currentAccessToken = cookieJar
                .getCookie("access_token", response.request.url)
                ?.value

            if (currentAccessToken != null && currentAccessToken != failingAccessToken) {
                // Token was already refreshed; strip stale Cookie header and retry
                return response.request.newBuilder()
                    .removeHeader("Cookie")
                    .build()
            }

            // 3. Perform synchronized refresh call
            val baseUrl = sessionManager.getServerUrl().toHttpUrlOrNull() ?: return null
            val refreshUrl = baseUrl.newBuilder()
                .addPathSegments("api/auth/refresh")
                .build()

            val refreshRequest = Request.Builder()
                .url(refreshUrl)
                .post("".toRequestBody())
                .build()

            return try {
                refreshClient.newCall(refreshRequest).execute().use { refreshResponse ->
                    when {
                        refreshResponse.isSuccessful -> {
                            response.request.newBuilder()
                                .removeHeader("Cookie")
                                .build()
                        }
                        refreshResponse.code == 401 || refreshResponse.code == 403 -> {
                            // Refresh token invalid, expired, or revoked: clear session
                            cookieJar.clear()
                            scope.launch {
                                sessionManager.setUnauthenticated()
                            }
                            null
                        }
                        else -> {
                            // Transient failure (429 rate-limit, 5xx server restart, etc.): preserve session
                            null
                        }
                    }
                }
            } catch (_: Exception) {
                null
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