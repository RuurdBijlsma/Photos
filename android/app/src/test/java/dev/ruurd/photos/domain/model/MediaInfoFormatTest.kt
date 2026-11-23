package dev.ruurd.photos.domain.model

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.util.Locale

class MediaInfoFormatTest {

    private val english = Locale.ENGLISH

    @Test
    fun `capture date renders the naive local timestamp`() {
        val details = MediaDetails(cloudId = "c1", takenAtLocal = "2024-05-03T17:04:05")

        assertEquals(
            "Friday 3 May 2024, 17:04:05",
            MediaInfoFormat.captureDateTime(details, english),
        )
    }

    @Test
    fun `offline capture date applies the stored timezone offset`() {
        val utcMay3At17 = 1_714_755_600_000L

        assertEquals(
            "Friday 3 May 2024, 19:00:00",
            MediaInfoFormat.offlineCaptureDateTime(utcMay3At17, 120, english),
        )
    }

    @Test
    fun `formats offsets and timezone labels`() {
        assertEquals("GMT+02:00", MediaInfoFormat.formatOffset(7200))
        assertEquals("GMT-05:30", MediaInfoFormat.formatOffset(-19_800))
        assertEquals("GMT+01:00 · Europe/Amsterdam", MediaInfoFormat.timezoneLabel(3600, "Europe/Amsterdam"))
        assertEquals("GMT+01:00", MediaInfoFormat.timezoneLabel(3600, null))
        assertNull(MediaInfoFormat.timezoneLabel(null, null))
    }

    @Test
    fun `formats size resolution and duration`() {
        assertNull(MediaInfoFormat.formatSize(0))
        assertEquals("512 B", MediaInfoFormat.formatSize(512))
        assertEquals("1.5 KB", MediaInfoFormat.formatSize(1536))
        assertEquals("5.0 MB", MediaInfoFormat.formatSize(5L * 1024 * 1024))
        assertEquals("4000 × 3000", MediaInfoFormat.formatResolution(4000, 3000))
        assertNull(MediaInfoFormat.formatResolution(0, 3000))
        assertEquals("1:05", MediaInfoFormat.formatDuration(65_000))
        assertEquals("1:01:01", MediaInfoFormat.formatDuration(3_661_000))
    }

    @Test
    fun `weather renders a freezing zero degrees reading and condition`() {
        assertEquals(
            "0 °C · Clear",
            MediaInfoFormat.weatherSummary(WeatherDetails(temperature = 0f, condition = "Clear")),
        )
        assertNull(MediaInfoFormat.weatherSummary(WeatherDetails()))
    }

    @Test
    fun `camera pills use photographic notation`() {
        assertEquals("1/250 s", MediaInfoFormat.exposureLabel(0.004f))
        assertEquals("2.5 s", MediaInfoFormat.exposureLabel(2.5f))
        assertEquals("f/1.8", MediaInfoFormat.apertureLabel(1.8f))
        assertEquals("ISO 400", MediaInfoFormat.isoLabel(400))
        assertNull(MediaInfoFormat.isoLabel(0))
        assertEquals("35 mm", MediaInfoFormat.focalLabel(null, 35f))
        assertEquals("Sony A7 III", MediaInfoFormat.cameraName(CameraDetails(make = "Sony", model = "A7 III")))
        assertEquals("Sony A7 III", MediaInfoFormat.cameraName(CameraDetails(make = "Sony", model = "Sony A7 III")))
    }

    @Test
    fun `location label and map links`() {
        val place = PlaceDetails(
            latitude = 52.09,
            longitude = 5.12,
            name = "Utrecht",
            admin1 = "Utrecht",
            countryName = "Netherlands",
        )

        assertEquals("Utrecht, Netherlands", MediaInfoFormat.locationLabel(place))
        assertEquals("52.09000, 5.12000", MediaInfoFormat.coordinates(52.09, 5.12))
        assertEquals("geo:52.09,5.12?q=52.09,5.12", MediaInfoFormat.geoUri(52.09, 5.12))
    }
}
