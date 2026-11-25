package dev.ruurd.photos.domain.model

import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MediaInfoUiModelTest {

    @Test
    fun `server details win over local info for file specs`() {
        val model = mediaInfoUiModel(
            details = MediaDetails(
                cloudId = "c1",
                fileName = "server.jpg",
                mimeType = "image/jpeg",
                sizeBytes = 1536L,
                width = 4000,
                height = 3000,
            ),
            localInfo = LocalMediaInfo(fileName = "local.jpg", sizeBytes = 999L, width = 10, height = 10),
            item = null,
        )

        assertEquals("server.jpg", model.fileName)
        assertEquals(
            listOf("12.0 MP", "4000 × 3000", "1.5 KB", "image/jpeg"),
            model.specChips,
        )
    }

    @Test
    fun `falls back to local info when the server has no details`() {
        val model = mediaInfoUiModel(
            details = null,
            localInfo = LocalMediaInfo(
                fileName = "local.jpg",
                width = 100,
                height = 50,
                latitude = 52.09,
                longitude = 5.12,
            ),
            item = null,
        )

        assertEquals("local.jpg", model.fileName)
        assertEquals(52.09, model.latitude)
        assertEquals("52.09000, 5.12000", model.coordinates)
        assertFalse(model.hasServerDetails)
        assertNull(model.caption)
    }

    @Test
    fun `camera card shows the 35mm equivalent only when it differs`() {
        val withEquivalent = mediaInfoUiModel(
            details = MediaDetails(
                cloudId = "c1",
                camera = CameraDetails(
                    make = "Sony",
                    model = "A7 III",
                    focalLength = 50f,
                    focalLengthIn35mm = 75f,
                ),
            ),
            localInfo = null,
            item = null,
        )

        assertEquals("Sony A7 III", withEquivalent.cameraName)
        assertEquals(
            listOf(DetailGroupCard.CAMERA_HEADER, DetailGroupCard.EXIF_METRICS),
            withEquivalent.detailCardTypes,
        )
        assertEquals(listOf("Focal" to "50 mm"), withEquivalent.primaryExifMetrics)
        assertEquals(listOf("35mm equiv." to "75 mm"), withEquivalent.secondaryExifMetrics)

        val withoutEquivalent = mediaInfoUiModel(
            details = MediaDetails(
                cloudId = "c1",
                camera = CameraDetails(focalLength = 35f, focalLengthIn35mm = 35f),
            ),
            localInfo = null,
            item = null,
        )
        assertTrue(withoutEquivalent.secondaryExifMetrics.isEmpty())
    }

    @Test
    fun `a cloud item with no metadata still shows its sync status`() {
        val item = TimelineMediaProjection(
            id = "cloud_c1",
            cloudId = "c1",
            localContentUri = null,
            yearMonth = 202405,
            ratio = 1.5f,
            isVideo = false,
            durationMs = null,
            hasThumbnails = true,
            syncStatus = SyncStatus.CLOUD_ONLY,
            sortTimestamp = 0L,
            timezoneOffsetMinutes = 0,
        )

        val model = mediaInfoUiModel(details = null, localInfo = null, item = item)

        assertEquals(SyncStatus.CLOUD_ONLY, model.syncStatus)
        assertTrue(model.hasDetailsSection)
        assertTrue(model.detailCardTypes.isEmpty())
    }
}
