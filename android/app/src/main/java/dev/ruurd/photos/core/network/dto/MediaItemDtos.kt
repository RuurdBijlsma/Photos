package dev.ruurd.photos.core.network.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * `GET /api/photos/{cloudId}/item`. The server serializes this struct without a rename rule, so
 * every key is snake_case (unlike `PUT /api/photos/{cloudId}/item`, which answers camelCase).
 */
@Serializable
data class MediaItemWithAlbumsDto(
    @SerialName("media_item") val mediaItem: FullMediaItemDto = FullMediaItemDto(),
    val albums: List<MediaItemAlbumRefDto> = emptyList(),
)

@Serializable
data class MediaItemAlbumRefDto(
    val id: String = "",
    val name: String = "",
    @SerialName("thumbnail_id") val thumbnailId: String? = null,
    @SerialName("media_count") val mediaCount: Int = 0,
)

@Serializable
data class FullMediaItemDto(
    val id: String = "",
    val filename: String = "",
    @SerialName("relative_path") val relativePath: String = "",
    @SerialName("missing_since") val missingSince: String? = null,
    val width: Int? = null,
    val height: Int? = null,
    val orientation: Int = 1,
    @SerialName("is_video") val isVideo: Boolean = false,
    @SerialName("duration_ms") val durationMs: Long? = null,
    @SerialName("taken_at_local") val takenAtLocal: String? = null,
    @SerialName("taken_at_utc") val takenAtUtc: String? = null,
    @SerialName("timezone_name") val timezoneName: String? = null,
    @SerialName("timezone_offset_seconds") val timezoneOffsetSeconds: Int? = null,
    @SerialName("use_panorama_viewer") val usePanoramaViewer: Boolean = false,
    val gps: GpsDto? = null,
    val weather: WeatherDto? = null,
    @SerialName("media_features") val mediaFeatures: MediaFeaturesDto? = null,
    @SerialName("camera_settings") val cameraSettings: CameraSettingsDto? = null,
    @SerialName("user_caption") val userCaption: String? = null,
)

/**
 * `PUT /api/photos/{cloudId}/item`. The server renames this struct to camelCase, and every field is
 * optional: leave the fields you are not editing at their defaults so they are omitted from the body.
 */
@Serializable
data class UpdateMediaItemRequestDto(
    val userCaption: PatchField<String> = PatchField.Ignore,
    /** Naive wall clock, `"yyyy-MM-dd'T'HH:mm:ss"`. */
    val takenAtLocal: String? = null,
    val timezoneOffsetSeconds: PatchField<Int> = PatchField.Ignore,
    val orientation: Int? = null,
    val gps: PatchField<GpsEditDto> = PatchField.Ignore,
)

@Serializable
data class GpsEditDto(
    val latitude: Double,
    val longitude: Double,
)

/** The only camelCase response: an orientation change assigns a new `cloudId`. */
@Serializable
data class UpdateMediaItemResponseDto(
    val mediaItemId: String = "",
)

@Serializable
data class GpsDto(
    val latitude: Double = 0.0,
    val longitude: Double = 0.0,
    val altitude: Double? = null,
    val location: LocationDto? = null,
)

@Serializable
data class LocationDto(
    val name: String = "",
    val admin1: String = "",
    val admin2: String = "",
    @SerialName("country_code") val countryCode: String = "",
    @SerialName("country_name") val countryName: String = "",
)

@Serializable
data class WeatherDto(
    val temperature: Float? = null,
    val condition: String? = null,
    @SerialName("wind_speed") val windSpeed: Float? = null,
    @SerialName("is_daytime") val isDaytime: Boolean? = null,
)

@Serializable
data class MediaFeaturesDto(
    @SerialName("mime_type") val mimeType: String? = null,
    @SerialName("size_bytes") val sizeBytes: Long? = null,
)

@Serializable
data class CameraSettingsDto(
    val iso: Int? = null,
    @SerialName("exposure_time") val exposureTime: Float? = null,
    val aperture: Float? = null,
    @SerialName("focal_length") val focalLength: Float? = null,
    @SerialName("focal_length_in_35mm") val focalLengthIn35mm: Float? = null,
    @SerialName("camera_make") val cameraMake: String? = null,
    @SerialName("camera_model") val cameraModel: String? = null,
    @SerialName("lens_make") val lensMake: String? = null,
    @SerialName("lens_model") val lensModel: String? = null,
    @SerialName("exposure_compensation") val exposureCompensation: Float? = null,
)
