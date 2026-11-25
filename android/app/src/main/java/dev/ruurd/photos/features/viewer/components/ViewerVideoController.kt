package dev.ruurd.photos.features.viewer.components

import android.content.Context
import android.net.Uri
import androidx.annotation.OptIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.HttpDataSource
import androidx.media3.datasource.okhttp.OkHttpDataSource
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import dev.ruurd.photos.core.network.VideoPreferences
import dev.ruurd.photos.domain.model.ResolvedVideoSource
import dev.ruurd.photos.domain.model.VideoPlaybackPolicy
import dev.ruurd.photos.domain.model.VideoQuality
import dev.ruurd.photos.domain.model.VideoUnavailableReason
import kotlinx.coroutines.delay
import okhttp3.OkHttpClient
import java.io.FileNotFoundException

/**
 * Owns the single shared [ExoPlayer] across the viewer session: lifecycle pause, OkHttp cookie/auth
 * streaming, quality-switch frame preservation, synchronous page-change stop, and isolated
 * high-frequency position state so 30Hz seekbar polling never recomposes the pager or video surface.
 */
@Stable
class ViewerVideoController @OptIn(UnstableApi::class) constructor(
    context: Context,
    okHttpClient: OkHttpClient,
    private val isOnlineProvider: () -> Boolean,
    private val onLocalVideoFailed: (mediaId: String) -> Unit,
    private val onCloudFallbackTo480p: (mediaId: String) -> Unit,
    private val onCloudVideoError: (mediaId: String, reason: VideoUnavailableReason) -> Unit,
) {
    val player: ExoPlayer

    var activeMediaId: String? by mutableStateOf(null)
        private set

    var activeCloudQuality: VideoQuality? by mutableStateOf(null)
        private set

    var isQualitySwitch: Boolean by mutableStateOf(false)
        private set

    var hasRenderedFirstFrame: Boolean by mutableStateOf(false)
        private set

    var isPlaying: Boolean by mutableStateOf(false)
        private set

    var isBuffering: Boolean by mutableStateOf(false)
        private set

    var isEnded: Boolean by mutableStateOf(false)
        private set

    var durationMs: Long by mutableLongStateOf(0L)
        private set

    private var positionMsState by mutableLongStateOf(0L)
    private var bufferedPositionMsState by mutableLongStateOf(0L)

    val positionMsProvider: () -> Long = { positionMsState }
    val bufferedPositionMsProvider: () -> Long = { bufferedPositionMsState }

    private var activeSourceKey: String? = null
    private var activeSource: ResolvedVideoSource? = null
    private var activeHasThumbnails: Boolean = true

    private val listener = object : Player.Listener {
        override fun onRenderedFirstFrame() {
            hasRenderedFirstFrame = true
            isQualitySwitch = false
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) {
            this@ViewerVideoController.isPlaying = isPlaying
            syncPositionFromPlayer()
        }

        override fun onPlaybackStateChanged(playbackState: Int) {
            isEnded = playbackState == Player.STATE_ENDED
            isBuffering = playbackState == Player.STATE_BUFFERING && player.playWhenReady
            val knownDuration = player.duration
            if (knownDuration != C.TIME_UNSET && knownDuration > 0L) {
                durationMs = knownDuration
            }
            if (playbackState == Player.STATE_ENDED && durationMs > 0L) {
                positionMsState = durationMs
            } else {
                syncPositionFromPlayer()
            }
        }

        override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
            isBuffering = player.playbackState == Player.STATE_BUFFERING && playWhenReady
        }

        override fun onPositionDiscontinuity(
            oldPosition: Player.PositionInfo,
            newPosition: Player.PositionInfo,
            reason: Int,
        ) {
            syncPositionFromPlayer()
        }

        override fun onPlayerError(error: PlaybackException) {
            val mediaId = activeMediaId ?: return
            val source = activeSource ?: return
            isBuffering = false

            when (source) {
                is ResolvedVideoSource.Local -> {
                    if (hasLocalAccessOrMissingCause(error) ||
                        error.errorCode in CODEC_OR_PARSING_ERROR_CODES
                    ) {
                        onLocalVideoFailed(mediaId)
                    } else {
                        onCloudVideoError(mediaId, VideoUnavailableReason.PLAYBACK_FAILED)
                    }
                }

                is ResolvedVideoSource.Cloud -> {
                    val isCodecOrParsing = error.errorCode in CODEC_OR_PARSING_ERROR_CODES
                    val isBadHttpStatus = error.errorCode == PlaybackException.ERROR_CODE_IO_BAD_HTTP_STATUS
                    val httpStatus = findInvalidResponseCode(error)
                    if (VideoPlaybackPolicy.shouldFallbackTo480p(
                            hasThumbnails = activeHasThumbnails,
                            currentQuality = source.quality,
                            isCodecOrParsingError = isCodecOrParsing,
                            isBadHttpStatus = isBadHttpStatus,
                            httpStatusCode = httpStatus,
                        )
                    ) {
                        onCloudFallbackTo480p(mediaId)
                    } else {
                        val reason = classifyCloudError(
                            error = error,
                            httpStatus = httpStatus,
                            hasThumbnails = activeHasThumbnails,
                            isOnline = isOnlineProvider(),
                        )
                        onCloudVideoError(mediaId, reason)
                    }
                }

                is ResolvedVideoSource.Unavailable -> Unit
            }
        }
    }

    init {
        val okHttpFactory = OkHttpDataSource.Factory(okHttpClient)
        val dataSourceFactory = DefaultDataSource.Factory(context.applicationContext, okHttpFactory)
        val loadControl = DefaultLoadControl.Builder()
            .setBufferDurationsMs(
                MIN_BUFFER_MS,
                MAX_BUFFER_MS,
                BUFFER_FOR_PLAYBACK_MS,
                BUFFER_FOR_PLAYBACK_AFTER_REBUFFER_MS,
            )
            .build()
        val audioAttributes = AudioAttributes.Builder()
            .setUsage(C.USAGE_MEDIA)
            .setContentType(C.AUDIO_CONTENT_TYPE_MOVIE)
            .build()

        player = ExoPlayer.Builder(context.applicationContext)
            .setMediaSourceFactory(DefaultMediaSourceFactory(dataSourceFactory))
            .setLoadControl(loadControl)
            .setAudioAttributes(audioAttributes, true)
            .setHandleAudioBecomingNoisy(true)
            .build()
            .apply {
                repeatMode = Player.REPEAT_MODE_OFF
                addListener(listener)
            }
    }

    fun applyPreferences(preferences: VideoPreferences) {
        val targetRepeat = if (preferences.isLoopEnabled) Player.REPEAT_MODE_ONE else Player.REPEAT_MODE_OFF
        if (player.repeatMode != targetRepeat) {
            player.repeatMode = targetRepeat
        }
        val targetVolume = if (preferences.isMuted) 0f else 1f
        if (player.volume != targetVolume) {
            player.volume = targetVolume
        }
        if (player.playbackParameters.speed != preferences.playbackSpeed) {
            player.setPlaybackSpeed(preferences.playbackSpeed)
        }
    }

    fun loadOrUpdateSource(
        mediaId: String,
        source: ResolvedVideoSource,
        hasThumbnails: Boolean,
        fallbackDurationMs: Long,
        autoPlay: Boolean,
    ) {
        if (source is ResolvedVideoSource.Unavailable) {
            stopAndClear()
            return
        }

        val uri = when (source) {
            is ResolvedVideoSource.Local -> Uri.parse(source.uri)
            is ResolvedVideoSource.Cloud -> Uri.parse(source.url)
        }
        val sourceKey = "$mediaId|$uri"
        activeHasThumbnails = hasThumbnails

        if (activeMediaId == mediaId && activeSourceKey == sourceKey) {
            return
        }

        val sameItemSwitch = activeMediaId == mediaId && activeSourceKey != null
        if (sameItemSwitch) {
            val resumePosMs = player.currentPosition.coerceAtLeast(0L)
            val resumePlayWhenReady = if (player.playbackState == Player.STATE_IDLE) {
                autoPlay
            } else {
                player.playWhenReady
            }
            isQualitySwitch = hasRenderedFirstFrame
            activeSourceKey = sourceKey
            activeSource = source
            activeCloudQuality = (source as? ResolvedVideoSource.Cloud)?.quality
            isBuffering = resumePlayWhenReady
            isEnded = false
            player.setMediaItem(MediaItem.fromUri(uri), resumePosMs)
            player.playWhenReady = resumePlayWhenReady
            player.prepare()
        } else {
            isQualitySwitch = false
            hasRenderedFirstFrame = false
            activeMediaId = mediaId
            activeSourceKey = sourceKey
            activeSource = source
            activeCloudQuality = (source as? ResolvedVideoSource.Cloud)?.quality
            positionMsState = 0L
            bufferedPositionMsState = 0L
            durationMs = fallbackDurationMs.coerceAtLeast(0L)
            isPlaying = false
            isEnded = false
            isBuffering = autoPlay
            player.stop()
            player.clearMediaItems()
            player.setMediaItem(MediaItem.fromUri(uri))
            player.playWhenReady = autoPlay
            player.prepare()
        }
    }

    fun stopAndClear() {
        if (activeMediaId == null && activeSourceKey == null && player.mediaItemCount == 0) {
            return
        }
        activeMediaId = null
        activeSourceKey = null
        activeSource = null
        activeCloudQuality = null
        isQualitySwitch = false
        hasRenderedFirstFrame = false
        isPlaying = false
        isBuffering = false
        isEnded = false
        positionMsState = 0L
        bufferedPositionMsState = 0L
        durationMs = 0L
        player.stop()
        player.clearMediaItems()
    }

    fun pause() {
        if (player.playWhenReady || player.isPlaying) {
            player.pause()
        }
    }

    fun togglePlayPause() {
        if (activeMediaId == null) return
        when {
            player.playbackState == Player.STATE_ENDED -> {
                positionMsState = 0L
                player.seekTo(0L)
                player.play()
            }

            player.isPlaying || (player.playWhenReady && player.playbackState == Player.STATE_BUFFERING) -> {
                player.pause()
            }

            else -> {
                if (player.playbackState == Player.STATE_IDLE && player.mediaItemCount > 0) {
                    player.prepare()
                }
                player.play()
            }
        }
    }

    fun seekTo(targetMs: Long) {
        if (activeMediaId == null) return
        val maxMs = durationMs.coerceAtLeast(0L)
        val clamped = if (maxMs > 0L) targetMs.coerceIn(0L, maxMs) else targetMs.coerceAtLeast(0L)
        positionMsState = clamped
        player.seekTo(clamped)
    }

    fun syncPositionFromPlayer() {
        if (activeMediaId == null) return
        val duration = player.duration
        if (duration != C.TIME_UNSET && duration > 0L) {
            durationMs = duration
        }
        val pos = player.currentPosition.coerceAtLeast(0L)
        positionMsState = if (durationMs > 0L) pos.coerceAtMost(durationMs) else pos
        val buffered = player.bufferedPosition.coerceAtLeast(0L)
        bufferedPositionMsState = if (durationMs > 0L) buffered.coerceAtMost(durationMs) else buffered
    }

    fun release() {
        player.removeListener(listener)
        player.release()
    }

    private fun findInvalidResponseCode(error: PlaybackException): Int? = generateSequence<Throwable?>(error.cause) { it.cause }
        .take(MAX_CAUSE_DEPTH)
        .filterIsInstance<HttpDataSource.InvalidResponseCodeException>()
        .firstOrNull()
        ?.responseCode

    private fun classifyCloudError(
        error: PlaybackException,
        httpStatus: Int?,
        hasThumbnails: Boolean,
        isOnline: Boolean,
    ): VideoUnavailableReason {
        if (!hasThumbnails && httpStatus == 404) {
            return VideoUnavailableReason.PROCESSING
        }
        if (httpStatus in MISSING_HTTP_CODES || error.errorCode == PlaybackException.ERROR_CODE_IO_FILE_NOT_FOUND) {
            return VideoUnavailableReason.MISSING_SOURCE
        }
        if (!isOnline ||
            error.errorCode == PlaybackException.ERROR_CODE_IO_NETWORK_CONNECTION_FAILED ||
            error.errorCode == PlaybackException.ERROR_CODE_IO_NETWORK_CONNECTION_TIMEOUT
        ) {
            return VideoUnavailableReason.OFFLINE
        }
        return VideoUnavailableReason.PLAYBACK_FAILED
    }

    companion object {
        private const val MIN_BUFFER_MS = 15_000
        private const val MAX_BUFFER_MS = 30_000
        private const val BUFFER_FOR_PLAYBACK_MS = 1_000
        private const val BUFFER_FOR_PLAYBACK_AFTER_REBUFFER_MS = 2_000
        private const val MAX_CAUSE_DEPTH = 8
        private val MISSING_HTTP_CODES = setOf(404, 415, 500)

        private val CODEC_OR_PARSING_ERROR_CODES = setOf(
            PlaybackException.ERROR_CODE_PARSING_CONTAINER_MALFORMED,
            PlaybackException.ERROR_CODE_PARSING_MANIFEST_MALFORMED,
            PlaybackException.ERROR_CODE_PARSING_CONTAINER_UNSUPPORTED,
            PlaybackException.ERROR_CODE_PARSING_MANIFEST_UNSUPPORTED,
            PlaybackException.ERROR_CODE_DECODER_INIT_FAILED,
            PlaybackException.ERROR_CODE_DECODER_QUERY_FAILED,
            PlaybackException.ERROR_CODE_DECODING_FAILED,
            PlaybackException.ERROR_CODE_DECODING_FORMAT_EXCEEDS_CAPABILITIES,
            PlaybackException.ERROR_CODE_DECODING_FORMAT_UNSUPPORTED,
            PlaybackException.ERROR_CODE_DECODING_RESOURCES_RECLAIMED,
            PlaybackException.ERROR_CODE_AUDIO_TRACK_INIT_FAILED,
            PlaybackException.ERROR_CODE_AUDIO_TRACK_WRITE_FAILED,
            PlaybackException.ERROR_CODE_AUDIO_TRACK_OFFLOAD_WRITE_FAILED,
            PlaybackException.ERROR_CODE_AUDIO_TRACK_OFFLOAD_INIT_FAILED,
        )

        internal fun hasLocalAccessOrMissingCause(error: PlaybackException): Boolean = generateSequence<Throwable?>(error.cause) { it.cause }
            .take(MAX_CAUSE_DEPTH)
            .any { it is SecurityException || it is FileNotFoundException }
    }
}

@Composable
fun rememberViewerVideoController(
    okHttpClient: OkHttpClient,
    isOnline: Boolean,
    preferences: VideoPreferences,
    isChromeVisible: Boolean,
    onLocalVideoFailed: (mediaId: String) -> Unit,
    onCloudFallbackTo480p: (mediaId: String) -> Unit,
    onCloudVideoError: (mediaId: String, reason: VideoUnavailableReason) -> Unit,
): ViewerVideoController {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val currentIsOnline by rememberUpdatedState(isOnline)
    val currentOnLocalFailed by rememberUpdatedState(onLocalVideoFailed)
    val currentOnFallback480p by rememberUpdatedState(onCloudFallbackTo480p)
    val currentOnCloudError by rememberUpdatedState(onCloudVideoError)

    val controller = remember(context, okHttpClient) {
        ViewerVideoController(
            context = context,
            okHttpClient = okHttpClient,
            isOnlineProvider = { currentIsOnline },
            onLocalVideoFailed = { currentOnLocalFailed(it) },
            onCloudFallbackTo480p = { currentOnFallback480p(it) },
            onCloudVideoError = { id, reason -> currentOnCloudError(id, reason) },
        )
    }

    LaunchedEffect(controller, preferences) {
        controller.applyPreferences(preferences)
    }

    // Only poll at 30Hz while chrome is visible and video is playing; sync immediately when shown.
    LaunchedEffect(controller, isChromeVisible, controller.isPlaying, controller.activeMediaId) {
        if (controller.activeMediaId == null) return@LaunchedEffect
        controller.syncPositionFromPlayer()
        if (isChromeVisible && controller.isPlaying) {
            while (true) {
                delay(PROGRESS_POLL_INTERVAL_MS)
                controller.syncPositionFromPlayer()
            }
        }
    }

    DisposableEffect(lifecycleOwner, controller) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP) {
                controller.pause()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    DisposableEffect(controller) {
        onDispose {
            controller.release()
        }
    }

    return controller
}

private const val PROGRESS_POLL_INTERVAL_MS = 33L
