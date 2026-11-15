package dev.ruurd.photos.data.datasource.local

import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.domain.model.LocalMediaFile
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.ZoneId

class LocalMediaMapperTest {

    private val zone = ZoneId.of("Europe/Amsterdam")

    private fun file(
        id: Long = 42L,
        width: Int = 4000,
        height: Int = 3000,
        orientation: Int = 0,
        dateTakenMs: Long = 1_730_000_000_000L,
        isVideo: Boolean = false,
        durationMs: Int? = null
    ) = LocalMediaFile(
        mediaStoreId = id,
        contentUri = "content://media/external/images/media/$id",
        bucketId = "1234",
        bucketName = "Camera",
        relativePath = "DCIM/Camera/",
        sizeBytes = 2_048L,
        width = width,
        height = height,
        orientation = orientation,
        dateTakenMs = dateTakenMs,
        mimeType = if (isVideo) "video/mp4" else "image/jpeg",
        isVideo = isVideo,
        durationMs = durationMs
    )

    @Test
    fun `ratio is orientation adjusted`() {
        assertEquals(4000f / 3000f, LocalMediaMapper.ratio(4000, 3000, 0), 0.0001f)
        assertEquals(3000f / 4000f, LocalMediaMapper.ratio(4000, 3000, 90), 0.0001f)
        assertEquals(3000f / 4000f, LocalMediaMapper.ratio(4000, 3000, 270), 0.0001f)
        assertEquals(4000f / 3000f, LocalMediaMapper.ratio(4000, 3000, 180), 0.0001f)
    }

    @Test
    fun `malformed dimensions fall back to a square ratio`() {
        assertEquals(1f, LocalMediaMapper.ratio(0, 3000, 0), 0f)
        assertEquals(1f, LocalMediaMapper.ratio(4000, -1, 0), 0f)
    }

    @Test
    fun `year month uses the device timezone wall clock`() {
        // 2024-10-31T23:23Z is already November in Amsterdam (UTC+1).
        val timestamp = 1_730_417_000_000L
        assertEquals(202411, LocalMediaMapper.yearMonth(timestamp, zone))
        assertEquals(202410, LocalMediaMapper.yearMonth(timestamp, ZoneId.of("UTC")))
    }

    @Test
    fun `local rows are never cloud rows`() {
        val entity = LocalMediaMapper.toEntity(file(), isVisibleInTimeline = true, zone = zone)

        assertEquals("local_42", entity.id)
        assertNull(entity.cloudId)
        assertNull(entity.blake3Hash)
        assertEquals(SyncStatus.LOCAL_ONLY, entity.syncStatus)
        assertEquals("content://media/external/images/media/42", entity.localContentUri)
        assertEquals(2_048L, entity.sizeBytes)
        assertEquals(true, entity.isVisibleInTimeline)
        assertEquals(202410, entity.yearMonth)
    }

    @Test
    fun `video duration is stored only for videos`() {
        assertNull(LocalMediaMapper.toEntity(file(), true, zone).durationMs)
        assertEquals(14_000, LocalMediaMapper.toEntity(file(isVideo = true, durationMs = 14_000), true, zone).durationMs)
    }
}
