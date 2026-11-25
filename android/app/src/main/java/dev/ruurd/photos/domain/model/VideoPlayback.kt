package dev.ruurd.photos.domain.model

import androidx.compose.runtime.Immutable
import java.util.Locale

enum class VideoQuality(val storageKey: String) {
    AUTO("auto"),
    ORIGINAL("original"),
    P480("480"),
    P144("144"),
    ;

    companion object {
        fun fromStorageKey(key: String?): VideoQuality = entries.firstOrNull { it.storageKey == key } ?: AUTO
    }
}

@Immutable
data class VideoQualityOption(
    val quality: VideoQuality,
    val label: String,
    val isEnabled: Boolean,
)

@Immutable
sealed interface ResolvedVideoSource {
    @Immutable
    data class Local(val uri: String) : ResolvedVideoSource

    @Immutable
    data class Cloud(
        val url: String,
        val quality: VideoQuality,
        val requiresAuth: Boolean,
    ) : ResolvedVideoSource

    @Immutable
    data class Unavailable(val reason: VideoUnavailableReason) : ResolvedVideoSource
}

enum class VideoUnavailableReason {
    MISSING_SOURCE,
    PROCESSING,
    OFFLINE,
    PLAYBACK_FAILED,
}

/**
 * Pure video source selection, quality labeling, and fallback rules so they stay unit-testable
 * outside Android/ExoPlayer.
 */
object VideoPlaybackPolicy {

    val PLAYBACK_SPEEDS = listOf(0.25f, 0.5f, 0.75f, 1.0f, 1.25f, 1.5f, 2.0f, 3.0f)

    private val FALLBACK_HTTP_STATUS_CODES = setOf(404, 415, 500)

    fun formatSpeed(speed: Float): String {
        val rounded = speed.toInt()
        return if (speed == rounded.toFloat()) {
            "${rounded}x"
        } else {
            val formatted = String.format(Locale.US, "%.2f", speed).trimEnd('0').trimEnd('.')
            "${formatted}x"
        }
    }

    fun formatVideoTime(positionMs: Long): String {
        val clampedSec = (positionMs.coerceAtLeast(0L)) / 1000L
        val hours = clampedSec / 3600L
        val minutes = (clampedSec % 3600L) / 60L
        val seconds = clampedSec % 60L
        return if (hours > 0L) {
            "%d:%02d:%02d".format(Locale.US, hours, minutes, seconds)
        } else {
            "%d:%02d".format(Locale.US, minutes, seconds)
        }
    }

    fun resolveCloudQuality(
        savedPreference: VideoQuality,
        hasThumbnails: Boolean,
        isOriginalMissing: Boolean,
        isUnmeteredWithoutDataSaver: Boolean,
        transientQualityOverride: VideoQuality?,
    ): VideoQuality {
        if (!hasThumbnails) return VideoQuality.ORIGINAL
        val requested = transientQualityOverride ?: when (savedPreference) {
            VideoQuality.AUTO -> if (isUnmeteredWithoutDataSaver) VideoQuality.ORIGINAL else VideoQuality.P480
            else -> savedPreference
        }
        return if (isOriginalMissing && requested == VideoQuality.ORIGINAL) {
            VideoQuality.P480
        } else {
            requested
        }
    }

    fun resolveSource(
        localContentUri: String?,
        localFailed: Boolean,
        cloudId: String?,
        baseUrl: String,
        hasThumbnails: Boolean,
        isOriginalMissing: Boolean,
        savedQuality: VideoQuality,
        transientQualityOverride: VideoQuality?,
        isUnmeteredWithoutDataSaver: Boolean,
    ): ResolvedVideoSource {
        if (!localContentUri.isNullOrBlank() && !localFailed) {
            return ResolvedVideoSource.Local(localContentUri)
        }
        if (cloudId.isNullOrBlank() || baseUrl.isBlank()) {
            return ResolvedVideoSource.Unavailable(VideoUnavailableReason.PLAYBACK_FAILED)
        }
        if (!hasThumbnails && isOriginalMissing) {
            return ResolvedVideoSource.Unavailable(VideoUnavailableReason.MISSING_SOURCE)
        }
        val quality = resolveCloudQuality(
            savedPreference = savedQuality,
            hasThumbnails = hasThumbnails,
            isOriginalMissing = isOriginalMissing,
            isUnmeteredWithoutDataSaver = isUnmeteredWithoutDataSaver,
            transientQualityOverride = transientQualityOverride,
        )
        return ResolvedVideoSource.Cloud(
            url = cloudVideoUrl(baseUrl, cloudId, quality),
            quality = quality,
            requiresAuth = quality == VideoQuality.ORIGINAL,
        )
    }

    fun cloudVideoUrl(baseUrl: String, cloudId: String, quality: VideoQuality): String = when (quality) {
        VideoQuality.ORIGINAL, VideoQuality.AUTO -> "$baseUrl/api/photos/$cloudId/video"
        VideoQuality.P480 -> "$baseUrl/thumbnails/$cloudId/480p.webm"
        VideoQuality.P144 -> "$baseUrl/thumbnails/$cloudId/144p.webm"
    }

    fun originalLabel(sourceHeight: Int?): String = if (sourceHeight != null && sourceHeight > 0) {
        "${sourceHeight}p (Original)"
    } else {
        "Original"
    }

    fun shortQualityLabel(quality: VideoQuality): String = when (quality) {
        VideoQuality.ORIGINAL, VideoQuality.AUTO -> "Orig"
        VideoQuality.P480 -> "480p"
        VideoQuality.P144 -> "144p"
    }

    fun availableQualities(
        hasThumbnails: Boolean,
        isOriginalMissing: Boolean,
        sourceHeight: Int?,
    ): List<VideoQualityOption> {
        val original = VideoQualityOption(
            quality = VideoQuality.ORIGINAL,
            label = originalLabel(sourceHeight),
            isEnabled = !isOriginalMissing,
        )
        if (!hasThumbnails) return listOf(original)
        return listOf(
            original,
            VideoQualityOption(quality = VideoQuality.P480, label = "480p", isEnabled = true),
            VideoQualityOption(quality = VideoQuality.P144, label = "144p", isEnabled = true),
        )
    }

    fun shouldFallbackTo480p(
        hasThumbnails: Boolean,
        currentQuality: VideoQuality,
        isCodecOrParsingError: Boolean,
        isBadHttpStatus: Boolean,
        httpStatusCode: Int?,
    ): Boolean {
        if (!hasThumbnails || currentQuality != VideoQuality.ORIGINAL) return false
        if (isCodecOrParsingError) return true
        return isBadHttpStatus && httpStatusCode != null && httpStatusCode in FALLBACK_HTTP_STATUS_CODES
    }
}