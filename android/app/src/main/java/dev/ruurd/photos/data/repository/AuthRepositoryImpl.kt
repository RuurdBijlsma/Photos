package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.cookie.PersistentCookieJar
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.User
import dev.ruurd.photos.domain.repository.AuthRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

class AuthRepositoryImpl(
    private val apiService: PhotosApiService,
    private val sessionManager: SessionManager,
    private val cookieJar: PersistentCookieJar,
    private val directClient: OkHttpClient,
    private val dispatchers: AppDispatchers
) : AuthRepository {

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

                directClient.newCall(request).execute().use { response ->
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
            try {
                apiService.logout()
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // Ignore network error on logout
            }
            cookieJar.clear()
            sessionManager.setUnauthenticated()
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