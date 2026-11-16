package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.cookie.PersistentCookieJar
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.domain.repository.AuthRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.Call
import okhttp3.Callback
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

class AuthRepositoryImpl(
    private val apiService: PhotosApiService,
    private val sessionManager: SessionManager,
    private val cookieJar: PersistentCookieJar,
    private val directClient: OkHttpClient,
    private val dispatchers: AppDispatchers,
    private val applicationScope: CoroutineScope
) : AuthRepository {

    // Reachability checks and best-effort logout should fail fast instead of waiting out 30s timeouts.
    private val shortTimeoutClient: OkHttpClient by lazy {
        directClient.newBuilder()
            .connectTimeout(5, TimeUnit.SECONDS)
            .readTimeout(5, TimeUnit.SECONDS)
            .writeTimeout(5, TimeUnit.SECONDS)
            .build()
    }

    override val authState: StateFlow<AuthState> = sessionManager.authState
    override val serverUrl: StateFlow<String> = sessionManager.serverUrl

    override suspend fun setServerUrl(url: String) {
        sessionManager.setServerUrl(url)
    }

    override suspend fun testServerConnection(url: String): Result<Unit> =
        withContext(dispatchers.io) {
            try {
                val normalized = SessionManager.normalizeUrl(url)
                if (normalized.isEmpty()) {
                    return@withContext Result.failure(IllegalArgumentException("Server URL cannot be empty"))
                }

                val healthUrl = "$normalized/api/health".toHttpUrlOrNull()
                    ?: return@withContext Result.failure(IllegalArgumentException("Invalid URL format"))

                val request = Request.Builder()
                    .url(healthUrl)
                    .get()
                    .build()

                // Enqueued instead of execute() so cancelling the caller aborts the socket instead of
                // blocking the IO dispatcher until the connect timeout expires.
                shortTimeoutClient.newCall(request).await().use { response ->
                    if (response.isSuccessful) {
                        Result.success(Unit)
                    } else {
                        Result.failure(Exception("Server returned status ${response.code}"))
                    }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Result.failure(e)
            }
        }

    override suspend fun login(email: String, password: String): Result<User> =
        withContext(dispatchers.io) {
            try {
                val jsonBody = JSONObject().apply {
                    put("email", email)
                    put("password", password)
                }.toString()

                val requestBody = jsonBody.toRequestBody("application/json".toMediaType())
                val response = apiService.login(requestBody)

                if (!response.isSuccessful) {
                    val errorMsg = when (response.code()) {
                        401 -> "Invalid email or password"
                        429 -> "Too many attempts. Please try again shortly"
                        else -> "Login failed: HTTP ${response.code()}"
                    }
                    return@withContext Result.failure(Exception(errorMsg))
                }

                // Successfully logged in; fetch user profile
                fetchCurrentUser()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Result.failure(e)
            }
        }

    override suspend fun logout(): Result<Unit> = withContext(dispatchers.io) {
        try {
            val baseUrl = sessionManager.getServerUrl().toHttpUrlOrNull()
            val logoutUrl = baseUrl?.newBuilder()?.addPathSegments("api/auth/logout")?.build()
            val cookieHeader = logoutUrl?.let { url ->
                cookieJar.loadForRequest(url)
                    .takeIf { it.isNotEmpty() }
                    ?.joinToString("; ") { "${it.name}=${it.value}" }
            }

            // Clear local auth immediately so sign out never hangs when the server is offline.
            cookieJar.clear()
            sessionManager.setUnauthenticated()

            if (logoutUrl != null) {
                applicationScope.launch(dispatchers.io) {
                    try {
                        val request = Request.Builder()
                            .url(logoutUrl)
                            .apply {
                                if (!cookieHeader.isNullOrEmpty()) {
                                    header("Cookie", cookieHeader)
                                }
                            }
                            .post("".toRequestBody())
                            .build()
                        shortTimeoutClient.newCall(request).await().close()
                    } catch (e: CancellationException) {
                        throw e
                    } catch (_: Exception) {
                        // Best-effort server token revocation; local session is already cleared.
                    }
                }
            }

            Result.success(Unit)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    override suspend fun fetchCurrentUser(): Result<User> = withContext(dispatchers.io) {
        try {
            val response = apiService.getMe()
            if (response.isSuccessful) {
                val bodyStr = response.body()?.string()
                    ?: return@withContext Result.failure(Exception("Empty response from /api/auth/me"))
                val json = JSONObject(bodyStr)
                val user = User(
                    id = json.getInt("id"),
                    email = json.getString("email"),
                    name = json.getString("name"),
                    role = json.optString("role", "user"),
                    mediaFolder = if (json.isNull("mediaFolder")) null else json.optString("mediaFolder"),
                    avatarId = if (json.isNull("avatarId")) null else json.optString("avatarId")
                )
                sessionManager.setAuthenticated(user)
                Result.success(user)
            } else {
                if (response.code() == 401) {
                    cookieJar.clear()
                    sessionManager.setUnauthenticated()
                }
                Result.failure(Exception("Failed to fetch user: HTTP ${response.code()}"))
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** Suspends on an enqueued call so cancellation cancels the request itself. */
private suspend fun Call.await(): Response = suspendCancellableCoroutine { continuation ->
    continuation.invokeOnCancellation { cancel() }
    enqueue(
        object : Callback {
            override fun onFailure(call: Call, e: IOException) {
                continuation.resumeWithException(e)
            }

            override fun onResponse(call: Call, response: Response) {
                continuation.resume(response)
            }
        }
    )
}