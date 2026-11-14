package dev.ruurd.photos.core.network

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import dev.ruurd.photos.domain.model.AuthState
import dev.ruurd.photos.domain.model.User
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

private val Context.sessionDataStore: DataStore<Preferences> by preferencesDataStore(name = "session_prefs")

class SessionManager(
    private val context: Context,
    scope: CoroutineScope
) : SyncTimestampStore {
    companion object {
        const val DEFAULT_SERVER_URL = "http://10.0.2.2:5272"

        private val KEY_SERVER_URL = stringPreferencesKey("server_url")
        private val KEY_USER_ID = intPreferencesKey("user_id")
        private val KEY_USER_EMAIL = stringPreferencesKey("user_email")
        private val KEY_USER_NAME = stringPreferencesKey("user_name")
        private val KEY_USER_ROLE = stringPreferencesKey("user_role")
        private val KEY_USER_MEDIA_FOLDER = stringPreferencesKey("user_media_folder")
        private val KEY_USER_AVATAR_ID = stringPreferencesKey("user_avatar_id")
        private val KEY_LATEST_SYNC_TIMESTAMP = longPreferencesKey("latest_sync_timestamp")

        fun normalizeUrl(url: String): String {
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
    }

    private val _serverUrl = MutableStateFlow(DEFAULT_SERVER_URL)
    val serverUrl: StateFlow<String> = _serverUrl.asStateFlow()

    private val _authState = MutableStateFlow<AuthState>(AuthState.Loading)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    private val _latestSyncTimestamp = MutableStateFlow(0L)
    override val latestSyncTimestamp: StateFlow<Long> = _latestSyncTimestamp.asStateFlow()

    private val initJob: Job = scope.launch {
        try {
            val prefs = context.sessionDataStore.data.first()
            val url = prefs[KEY_SERVER_URL] ?: DEFAULT_SERVER_URL
            _serverUrl.value = normalizeUrl(url).ifEmpty { DEFAULT_SERVER_URL }
            _latestSyncTimestamp.value = prefs[KEY_LATEST_SYNC_TIMESTAMP] ?: 0L

            val userId = prefs[KEY_USER_ID]
            val email = prefs[KEY_USER_EMAIL]
            val name = prefs[KEY_USER_NAME]
            val role = prefs[KEY_USER_ROLE]

            _authState.value = if (userId != null && email != null && name != null && role != null) {
                AuthState.Authenticated(
                    User(
                        id = userId,
                        email = email,
                        name = name,
                        role = role,
                        mediaFolder = prefs[KEY_USER_MEDIA_FOLDER],
                        avatarId = prefs[KEY_USER_AVATAR_ID]
                    )
                )
            } else {
                AuthState.Unauthenticated
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            _authState.value = AuthState.Unauthenticated
        }
    }

    suspend fun setServerUrl(url: String) {
        initJob.join()
        val cleanUrl = normalizeUrl(url)
        _serverUrl.value = cleanUrl
        context.sessionDataStore.edit { prefs ->
            prefs[KEY_SERVER_URL] = cleanUrl
        }
    }

    suspend fun setAuthenticated(user: User) {
        initJob.join()
        _authState.value = AuthState.Authenticated(user)
        context.sessionDataStore.edit { prefs ->
            prefs[KEY_USER_ID] = user.id
            prefs[KEY_USER_EMAIL] = user.email
            prefs[KEY_USER_NAME] = user.name
            prefs[KEY_USER_ROLE] = user.role
            if (user.mediaFolder != null) {
                prefs[KEY_USER_MEDIA_FOLDER] = user.mediaFolder
            } else {
                prefs.remove(KEY_USER_MEDIA_FOLDER)
            }
            if (user.avatarId != null) {
                prefs[KEY_USER_AVATAR_ID] = user.avatarId
            } else {
                prefs.remove(KEY_USER_AVATAR_ID)
            }
        }
    }

    suspend fun setUnauthenticated() {
        initJob.join()
        _authState.value = AuthState.Unauthenticated
        _latestSyncTimestamp.value = 0L
        context.sessionDataStore.edit { prefs ->
            prefs.remove(KEY_USER_ID)
            prefs.remove(KEY_USER_EMAIL)
            prefs.remove(KEY_USER_NAME)
            prefs.remove(KEY_USER_ROLE)
            prefs.remove(KEY_USER_MEDIA_FOLDER)
            prefs.remove(KEY_USER_AVATAR_ID)
            prefs.remove(KEY_LATEST_SYNC_TIMESTAMP)
        }
    }

    override suspend fun setLatestSyncTimestamp(timestamp: Long) {
        initJob.join()
        _latestSyncTimestamp.value = timestamp
        context.sessionDataStore.edit { prefs ->
            prefs[KEY_LATEST_SYNC_TIMESTAMP] = timestamp
        }
    }

    fun getServerUrl(): String = _serverUrl.value
}