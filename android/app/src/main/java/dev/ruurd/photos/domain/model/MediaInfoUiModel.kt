package dev.ruurd.photos.domain.model

import androidx.compose.runtime.Immutable
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlin.math.roundToInt

/** Groups that are rendered as one visually joined card stack in the info panel. */
enum class DetailGroupCard {
    CAMERA_HEADER,
    EXIF_METRICS,
    FILE_SPECS,
}

/**
 * Everything the info panel renders, already formatted. Built off the composition so the panel stays
 * declarative and the fallback rules (server → local → MediaStore) stay unit-testable.
 */
@Immutable
data class MediaInfoUiModel(
    val captureDateTime: String?,
    val timezone: String?,
    val caption: String?,
    val albums: List<AlbumRef>,
    val latitude: Double?,
    val longitude: Double?,
    val placeLabel: String?,
    val coordinates: String?,
    val syncStatus: SyncStatus?,
    val hasCloudId: Boolean,
    val hasServerDetails: Boolean,
    val isOnline: Boolean,
    val isDetailsLoading: Boolean,
    val hasDetailsFailed: Boolean,
    val cameraName: String?,
    val lensName: String?,
    val primaryExifMetrics: List<Pair<String, String>>,
    val secondaryExifMetrics: List<Pair<String, String>>,
    val fileName: String?,
    val specChips: List<String>,
    val weather: String?,
    val windSummary: String?,
    val isDaytime: Boolean,
    val detailCardTypes: List<DetailGroupCard>,
) {
    val hasDetailsSection: Boolean
        get() = syncStatus != null ||
            detailCardTypes.isNotEmpty() ||
            weather != null ||
            windSummary != null
}

fun mediaInfoUiModel(
    details: MediaDetails?,
    localInfo: LocalMediaInfo?,
    item: TimelineMediaProjection?,
    isOnline: Boolean = true,
    isDetailsLoading: Boolean = false,
    hasDetailsFailed: Boolean = false,
): MediaInfoUiModel {
    val captureDateTime = MediaInfoFormat.resolveLocalDateTime(details, item, localInfo)
        ?.let(MediaInfoFormat::formatCompactDateTime)
    val offsetSeconds = details?.timezoneOffsetSeconds
        ?: item?.takeIf { it.cloudId != null }?.timezoneOffsetMinutes?.times(60)
    val fileName = details?.fileName ?: localInfo?.fileName
    val width = details?.width ?: localInfo?.width
    val height = details?.height ?: localInfo?.height
    val duration = MediaInfoFormat.formatDuration(details?.durationMs ?: localInfo?.durationMs)
    val mimeType = details?.mimeType ?: localInfo?.mimeType

    val camera = details?.camera ?: localInfo?.camera
    val cameraName = MediaInfoFormat.cameraName(camera)
    val lensName = MediaInfoFormat.lensName(camera)
    val aperture = MediaInfoFormat.apertureLabel(camera?.aperture)
    val shutter = MediaInfoFormat.exposureLabel(camera?.exposureTime)
    val focal = MediaInfoFormat.focalLabel(camera?.focalLength, camera?.focalLengthIn35mm)
    val iso = MediaInfoFormat.isoLabel(camera?.iso)
    val exposureBias = MediaInfoFormat.exposureCompensationLabel(camera?.exposureCompensation)
    // Only worth a line when the 35mm equivalent actually differs from the real focal length.
    val focal35mm = camera?.focalLengthIn35mm
        ?.takeIf { equivalent ->
            equivalent > 0f &&
                camera.focalLength?.takeIf { it > 0f }
                    ?.let { it.roundToInt() != equivalent.roundToInt() } == true
        }
        ?.let { "${it.roundToInt()} mm" }

    val primaryExifMetrics = listOfNotNull(
        aperture?.let { "Aperture" to it },
        shutter?.let { "Shutter" to it },
        focal?.let { "Focal" to it },
        iso?.let { "ISO" to it },
    )
    val secondaryExifMetrics = listOfNotNull(
        exposureBias?.let { "Exposure bias" to it },
        focal35mm?.let { "35mm equiv." to it },
    )

    val place = details?.place
    val latitude = place?.latitude ?: localInfo?.latitude
    val longitude = place?.longitude ?: localInfo?.longitude
    val specChips = listOfNotNull(
        MediaInfoFormat.formatMegapixels(width, height),
        MediaInfoFormat.formatResolution(width, height),
        MediaInfoFormat.formatSize(details?.sizeBytes ?: localInfo?.sizeBytes),
        duration,
        mimeType,
    )

    val detailCardTypes = buildList {
        if (cameraName != null || lensName != null) add(DetailGroupCard.CAMERA_HEADER)
        if (primaryExifMetrics.isNotEmpty() || secondaryExifMetrics.isNotEmpty()) {
            add(DetailGroupCard.EXIF_METRICS)
        }
        if (fileName != null || specChips.isNotEmpty()) add(DetailGroupCard.FILE_SPECS)
    }

    return MediaInfoUiModel(
        captureDateTime = captureDateTime,
        timezone = MediaInfoFormat.timezoneLabel(offsetSeconds, details?.timezoneName),
        caption = details?.userCaption,
        albums = details?.albums.orEmpty(),
        latitude = latitude,
        longitude = longitude,
        placeLabel = MediaInfoFormat.locationLabel(place),
        coordinates = MediaInfoFormat.coordinates(latitude, longitude),
        syncStatus = item?.syncStatus,
        hasCloudId = item?.cloudId != null,
        hasServerDetails = details != null,
        isOnline = isOnline,
        isDetailsLoading = isDetailsLoading,
        hasDetailsFailed = hasDetailsFailed,
        cameraName = cameraName,
        lensName = lensName,
        primaryExifMetrics = primaryExifMetrics,
        secondaryExifMetrics = secondaryExifMetrics,
        fileName = fileName,
        specChips = specChips,
        weather = MediaInfoFormat.weatherSummary(details?.weather),
        windSummary = MediaInfoFormat.windSpeedLabel(details?.weather?.windSpeed),
        isDaytime = details?.weather?.isDaytime != false,
        detailCardTypes = detailCardTypes,
    )
}
