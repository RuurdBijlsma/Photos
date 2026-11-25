package dev.ruurd.photos.core.network

import android.content.Context
import androidx.compose.runtime.Immutable
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import dev.ruurd.photos.domain.model.VideoQuality
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.withContext

private val Context.videoDataStore: DataStore<Preferences> by preferencesDataStore(name = "video_prefs")

@Immutable
data class VideoPreferences(
    val isLoopEnabled: Boolean = false,
    val playbackSpeed: Float = 1.0f,
    val isMuted: Boolean = false,
    val preferredQuality: VideoQuality = VideoQuality.AUTO,
)

interface VideoPreferencesSource {
    val preferences: StateFlow<VideoPreferences>

    suspend fun setLoopEnabled(enabled: Boolean)
    suspend fun setPlaybackSpeed(speed: Float)
    suspend fun setMuted(muted: Boolean)
    suspend fun setPreferredQuality(quality: VideoQuality)
}

class VideoPreferencesStore(
    private val context: Context,
    scope: CoroutineScope,
) : VideoPreferencesSource {

    private val _preferences = MutableStateFlow(VideoPreferences())
    override val preferences: StateFlow<VideoPreferences> = _preferences.asStateFlow()

    init {
        context.videoDataStore.data
            .catch { /* keep defaults on read failure */ }
            .onEach { prefs ->
                _preferences.value = VideoPreferences(
                    isLoopEnabled = prefs[KEY_LOOP_ENABLED] ?: false,
                    playbackSpeed = prefs[KEY_PLAYBACK_SPEED] ?: 1.0f,
                    isMuted = prefs[KEY_MUTED] ?: false,
                    preferredQuality = VideoQuality.fromStorageKey(prefs[KEY_QUALITY]),
                )
            }
            .launchIn(scope)
    }

    override suspend fun setLoopEnabled(enabled: Boolean) {
        _preferences.update { it.copy(isLoopEnabled = enabled) }
        withContext(NonCancellable) {
            context.videoDataStore.edit { it[KEY_LOOP_ENABLED] = enabled }
        }
    }

    override suspend fun setPlaybackSpeed(speed: Float) {
        val clamped = speed.coerceIn(0.25f, 3.0f)
        _preferences.update { it.copy(playbackSpeed = clamped) }
        withContext(NonCancellable) {
            context.videoDataStore.edit { it[KEY_PLAYBACK_SPEED] = clamped }
        }
    }

    override suspend fun setMuted(muted: Boolean) {
        _preferences.update { it.copy(isMuted = muted) }
        withContext(NonCancellable) {
            context.videoDataStore.edit { it[KEY_MUTED] = muted }
        }
    }

    override suspend fun setPreferredQuality(quality: VideoQuality) {
        _preferences.update { it.copy(preferredQuality = quality) }
        withContext(NonCancellable) {
            context.videoDataStore.edit { it[KEY_QUALITY] = quality.storageKey }
        }
    }

    private companion object {
        val KEY_LOOP_ENABLED = booleanPreferencesKey("loop_enabled")
        val KEY_PLAYBACK_SPEED = floatPreferencesKey("playback_speed")
        val KEY_MUTED = booleanPreferencesKey("muted")
        val KEY_QUALITY = stringPreferencesKey("preferred_quality")
    }
}