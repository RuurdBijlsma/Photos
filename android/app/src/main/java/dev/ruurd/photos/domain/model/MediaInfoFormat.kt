package dev.ruurd.photos.domain.model

import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import java.time.Instant
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlin.math.abs
import kotlin.math.roundToInt

/**
 * Pure formatting for the viewer's top bar and info sheet, so the Compose layer never formats on
 * the main thread during scrolls and the rules stay unit-testable.
 */
object MediaInfoFormat {

    private const val COMPACT_DATE_TIME_PATTERN = "EEE, d MMM yyyy '•' HH:mm"
    private const val VIEWER_TOP_DATE_PATTERN = "d MMM yyyy"
    private const val VIEWER_TOP_TIME_PATTERN = "HH:mm"

    fun resolveLocalDateTime(
        details: MediaDetails?,
        item: TimelineMediaProjection?,
        localInfo: LocalMediaInfo?,
    ): LocalDateTime? {
        details?.takenAtLocal?.let(::parseNaiveLocal)?.let { return it }
        if (item != null && item.sortTimestamp > 0L) {
            return LocalDateTime.ofInstant(
                Instant.ofEpochMilli(item.sortTimestamp + item.timezoneOffsetMinutes * 60_000L),
                ZoneOffset.UTC,
            )
        }
        localInfo?.dateTakenMs?.takeIf { it > 0L }?.let { epoch ->
            return LocalDateTime.ofInstant(Instant.ofEpochMilli(epoch), ZoneId.systemDefault())
        }
        return null
    }

    fun formatCompactDateTime(
        dateTime: LocalDateTime,
        locale: Locale = Locale.getDefault(),
    ): String = dateTime.format(DateTimeFormatter.ofPattern(COMPACT_DATE_TIME_PATTERN, locale))

    fun captureDateTime(details: MediaDetails?, locale: Locale = Locale.getDefault()): String? {
        val local = details?.takenAtLocal ?: return null
        val parsed = parseNaiveLocal(local) ?: return null
        return formatCompactDateTime(parsed, locale)
    }

    /** Offline wall clock: the UTC instant shifted by the item's stored capture-time offset. */
    fun offlineCaptureDateTime(
        sortTimestamp: Long,
        timezoneOffsetMinutes: Int,
        locale: Locale = Locale.getDefault(),
    ): String = formatCompactDateTime(
        LocalDateTime.ofInstant(
            Instant.ofEpochMilli(sortTimestamp + timezoneOffsetMinutes * 60_000L),
            ZoneOffset.UTC,
        ),
        locale,
    )

    /** MediaStore `DATE_TAKEN` (UTC epoch) rendered in the device's timezone. */
    fun epochDateTime(epochMillis: Long, locale: Locale = Locale.getDefault()): String = formatCompactDateTime(
        LocalDateTime.ofInstant(Instant.ofEpochMilli(epochMillis), ZoneId.systemDefault()),
        locale,
    )

    fun viewerTopTitle(
        details: MediaDetails?,
        item: TimelineMediaProjection?,
        localInfo: LocalMediaInfo?,
        locale: Locale = Locale.getDefault(),
    ): String? {
        val dateTime = resolveLocalDateTime(details, item, localInfo)
        return dateTime?.format(DateTimeFormatter.ofPattern(VIEWER_TOP_DATE_PATTERN, locale))
            ?: shortPlaceName(details?.place)
    }

    fun viewerTopSubtitle(
        details: MediaDetails?,
        item: TimelineMediaProjection?,
        localInfo: LocalMediaInfo?,
        locale: Locale = Locale.getDefault(),
    ): String? {
        val dateTime = resolveLocalDateTime(details, item, localInfo) ?: return null
        val timeLabel = dateTime.format(DateTimeFormatter.ofPattern(VIEWER_TOP_TIME_PATTERN, locale))
        val placeName = shortPlaceName(details?.place)
        return if (placeName != null) "$timeLabel • $placeName" else timeLabel
    }

    fun shortPlaceName(place: PlaceDetails?): String? = listOf(place?.name, place?.admin1, place?.countryName)
        .firstOrNull { !it.isNullOrBlank() }
        ?.trim()

    fun timezoneLabel(offsetSeconds: Int?, name: String?): String? {
        val offset = offsetSeconds?.let(::formatOffset)
        return when {
            offset != null && !name.isNullOrBlank() -> "$offset · $name"
            offset != null -> offset
            !name.isNullOrBlank() -> name
            else -> null
        }
    }

    fun formatOffset(offsetSeconds: Int): String {
        val sign = if (offsetSeconds < 0) "-" else "+"
        val totalMinutes = abs(offsetSeconds) / 60
        return "GMT$sign%02d:%02d".format(totalMinutes / 60, totalMinutes % 60)
    }

    fun formatSize(bytes: Long?): String? {
        if (bytes == null || bytes <= 0L) return null
        val units = arrayOf("B", "KB", "MB", "GB", "TB")
        var value = bytes.toDouble()
        var unit = 0
        while (value >= 1024.0 && unit < units.lastIndex) {
            value /= 1024.0
            unit++
        }
        return if (unit == 0) "$bytes B" else "%.1f %s".format(value, units[unit])
    }

    fun formatResolution(width: Int?, height: Int?): String? {
        if (width == null || height == null || width <= 0 || height <= 0) return null
        return "$width × $height"
    }

    fun formatMegapixels(width: Int?, height: Int?): String? {
        if (width == null || height == null || width <= 0 || height <= 0) return null
        val megapixels = (width.toLong() * height.toLong()) / 1_000_000.0
        return if (megapixels >= 0.1) "%.1f MP".format(Locale.US, megapixels) else null
    }

    fun formatDuration(durationMs: Long?): String? {
        if (durationMs == null || durationMs <= 0L) return null
        val totalSeconds = durationMs / 1000
        val hours = totalSeconds / 3600
        val minutes = (totalSeconds % 3600) / 60
        val seconds = totalSeconds % 60
        return if (hours > 0) {
            "%d:%02d:%02d".format(hours, minutes, seconds)
        } else {
            "%d:%02d".format(minutes, seconds)
        }
    }

    /** `temperature != null`, not truthy, so a freezing `0 °C` shot still shows its weather. */
    fun weatherSummary(weather: WeatherDetails?): String? {
        if (weather == null) return null
        val temperature = weather.temperature?.let { "${it.roundToInt()} °C" }
        val condition = weather.condition?.takeIf { it.isNotBlank() }
        return when {
            temperature != null && condition != null -> "$temperature · $condition"
            temperature != null -> temperature
            condition != null -> condition
            else -> null
        }
    }

    fun exposureLabel(exposureTime: Float?): String? {
        if (exposureTime == null || exposureTime <= 0f) return null
        if (exposureTime >= 1f) {
            val seconds = String.format(Locale.US, "%.1f", exposureTime).removeSuffix(".0")
            return "$seconds s"
        }
        return "1/${(1f / exposureTime).roundToInt()} s"
    }

    fun apertureLabel(aperture: Float?): String? {
        if (aperture == null || aperture <= 0f) return null
        return "f/${String.format(Locale.US, "%.1f", aperture).removeSuffix(".0")}"
    }

    fun isoLabel(iso: Int?): String? = if (iso == null || iso <= 0) null else "ISO $iso"

    fun focalLabel(focalLength: Float?, focalLengthIn35mm: Float?): String? {
        val value = focalLength?.takeIf { it > 0f } ?: focalLengthIn35mm?.takeIf { it > 0f } ?: return null
        return "${value.roundToInt()} mm"
    }

    fun exposureCompensationLabel(exposureCompensation: Float?): String? {
        if (exposureCompensation == null) return null
        val rounded = String.format(Locale.US, "%.1f", exposureCompensation).removeSuffix(".0")
        return if (exposureCompensation > 0f) "+$rounded EV" else "$rounded EV"
    }

    fun cameraName(camera: CameraDetails?): String? {
        if (camera == null) return null
        val make = camera.make?.trim()?.takeIf { it.isNotEmpty() }
        val model = camera.model?.trim()?.takeIf { it.isNotEmpty() }
        return when {
            make != null && model != null && !model.contains(make, ignoreCase = true) -> "$make $model"
            model != null -> model
            make != null -> make
            else -> null
        }
    }

    fun lensName(camera: CameraDetails?): String? {
        if (camera == null) return null
        val make = camera.lensMake?.trim()?.takeIf { it.isNotEmpty() }
        val model = camera.lensModel?.trim()?.takeIf { it.isNotEmpty() }
        return when {
            make != null && model != null && !model.contains(make, ignoreCase = true) -> "$make $model"
            model != null -> model
            make != null -> make
            else -> null
        }
    }

    fun locationLabel(place: PlaceDetails?): String? {
        if (place == null) return null
        val parts = listOf(place.name, place.admin1, place.countryName)
            .mapNotNull { it?.trim()?.takeIf(String::isNotEmpty) }
            .distinct()
        return parts.takeIf { it.isNotEmpty() }?.joinToString(", ")
    }

    fun coordinates(latitude: Double?, longitude: Double?): String? {
        if (latitude == null || longitude == null) return null
        return "%.5f, %.5f".format(Locale.US, latitude, longitude)
    }

    /** `geo:` intent, with a web map fallback for devices with no maps app. */
    fun geoUri(latitude: Double, longitude: Double): String = "geo:$latitude,$longitude?q=$latitude,$longitude"

    fun webMapUrl(latitude: Double, longitude: Double): String = "https://www.openstreetmap.org/?mlat=$latitude&mlon=$longitude"

    /** `"yyyy-MM-dd'T'HH:mm:ss"` without a zone, matching the server's `NaiveDateTime`. */
    fun parseNaiveLocal(value: String): LocalDateTime? = try {
        LocalDateTime.parse(value.replace(' ', 'T'))
    } catch (_: Exception) {
        null
    }
}
