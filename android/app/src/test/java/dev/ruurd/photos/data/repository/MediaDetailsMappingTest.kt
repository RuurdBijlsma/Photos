package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.network.dto.CameraSettingsDto
import dev.ruurd.photos.core.network.dto.FullMediaItemDto
import dev.ruurd.photos.core.network.dto.GpsDto
import dev.ruurd.photos.core.network.dto.LocationDto
import dev.ruurd.photos.core.network.dto.MediaFeaturesDto
import dev.ruurd.photos.core.network.dto.MediaItemAlbumRefDto
import dev.ruurd.photos.core.network.dto.MediaItemWithAlbumsDto
import dev.ruurd.photos.core.network.dto.WeatherDto
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MediaDetailsMappingTest {

    @Test
    fun `maps a full item and album refs`() {
        val dto = MediaItemWithAlbumsDto(
            mediaItem = FullMediaItemDto(
                id = "c1",
                filename = "IMG_0001.jpg",
                width = 4000,
                height = 3000,
                takenAtLocal = "2024-05-03T17:04:05",
                timezoneName = "Europe/Amsterdam",
                timezoneOffsetSeconds = 3600,
                userCaption = "  ",
                mediaFeatures = MediaFeaturesDto(mimeType = "image/jpeg", sizeBytes = 2048),
                gps = GpsDto(
                    latitude = 52.09,
                    longitude = 5.12,
                    location = LocationDto(
                        name = "Utrecht",
                        admin1 = "Utrecht",
                        countryCode = "NL",
                        countryName = "N/A",
                    ),
                ),
                weather = WeatherDto(temperature = 0f, condition = "Clear", windSpeed = 3f, isDaytime = false),
                cameraSettings = CameraSettingsDto(
                    cameraMake = "Sony",
                    cameraModel = "A7 III",
                    aperture = 1.8f,
                ),
            ),
            albums = listOf(
                MediaItemAlbumRefDto(id = "a1", name = "Trips"),
                MediaItemAlbumRefDto(id = "a2", name = " "),
            ),
        )

        val details = dto.toDomain("c1")

        assertEquals("IMG_0001.jpg", details.fileName)
        assertEquals("image/jpeg", details.mimeType)
        assertEquals(2048L, details.sizeBytes)
        assertEquals(4000, details.width)
        assertNull(details.userCaption)
        assertEquals("Sony", details.camera?.make)
        assertEquals("Utrecht", details.place?.name)
        assertNull(details.place?.countryName)
        assertEquals(0f, details.weather?.temperature)
        assertEquals(false, details.weather?.isDaytime)
        assertEquals(listOf("a1"), details.albums.map { it.id })
        assertTrue(!details.isMissing)
    }

    @Test
    fun `missing items and blank names degrade to nulls`() {
        val dto = MediaItemWithAlbumsDto(
            mediaItem = FullMediaItemDto(
                id = "c2",
                filename = " ",
                missingSince = "2024-06-01T00:00:00",
                cameraSettings = CameraSettingsDto(),
            ),
        )

        val details = dto.toDomain("c2")

        assertNull(details.fileName)
        assertNull(details.camera)
        assertTrue(details.isMissing)
    }
}
