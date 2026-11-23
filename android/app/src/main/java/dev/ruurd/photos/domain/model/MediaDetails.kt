package dev.ruurd.photos.domain.model

import androidx.compose.runtime.Immutable

/**
 * `GET /api/photos/{cloudId}/item` for one item, flattened to what the viewer's info sheet shows.
 */
@Immutable
data class MediaDetails(
    val cloudId: String,
    val fileName: String? = null,
    val mimeType: String? = null,
    val sizeBytes: Long? = null,
    val width: Int? = null,
    val height: Int? = null,
    val isVideo: Boolean = false,
    val durationMs: Long? = null,
    /** Naive wall-clock capture time, `"yyyy-MM-dd'T'HH:mm:ss"`. */
    val takenAtLocal: String? = null,
    val timezoneOffsetSeconds: Int? = null,
    val timezoneName: String? = null,
    val missingSince: String? = null,
    val userCaption: String? = null,
    val orientation: Int = 1,
    val camera: CameraDetails? = null,
    val weather: WeatherDetails? = null,
    val place: PlaceDetails? = null,
    val albums: List<AlbumRef> = emptyList(),
) {
    val isMissing: Boolean get() = missingSince != null
}

/**
 * [LocalMediaInfo] comes from `MediaStore` + `ExifInterface` for items physically on the phone, so
 * a `LOCAL_ONLY` item or an item whose `/item` fetch failed offline still renders an info sheet.
 */
@Immutable
data class LocalMediaInfo(
    val fileName: String? = null,
    val mimeType: String? = null,
    val sizeBytes: Long? = null,
    val width: Int? = null,
    val height: Int? = null,
    val dateTakenMs: Long? = null,
    val durationMs: Long? = null,
    val camera: CameraDetails? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
)

@Immutable
data class CameraDetails(
    val make: String? = null,
    val model: String? = null,
    val lensMake: String? = null,
    val lensModel: String? = null,
    val exposureTime: Float? = null,
    val aperture: Float? = null,
    val iso: Int? = null,
    val focalLength: Float? = null,
    val focalLengthIn35mm: Float? = null,
    val exposureCompensation: Float? = null,
) {
    val isEmpty: Boolean
        get() = make == null && model == null && lensMake == null && lensModel == null &&
            exposureTime == null && aperture == null && iso == null && focalLength == null &&
            focalLengthIn35mm == null && exposureCompensation == null
}

@Immutable
data class WeatherDetails(
    val temperature: Float? = null,
    val condition: String? = null,
    val windSpeed: Float? = null,
    /** `null` when the server has no sun data, which is treated as daytime. */
    val isDaytime: Boolean? = null,
)

@Immutable
data class PlaceDetails(
    val latitude: Double,
    val longitude: Double,
    val name: String? = null,
    val admin1: String? = null,
    val countryName: String? = null,
)

@Immutable
data class AlbumRef(
    val id: String,
    val name: String,
)
