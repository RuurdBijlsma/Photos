package dev.ruurd.photos.domain.model

import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MediaMutationPolicyTest {

    private fun item(
        id: String = "cloud_c1",
        cloudId: String? = "c1",
        localContentUri: String? = null,
        isVideo: Boolean = false,
        syncStatus: SyncStatus = SyncStatus.CLOUD_ONLY,
    ) = TimelineMediaProjection(
        id = id,
        cloudId = cloudId,
        localContentUri = localContentUri,
        yearMonth = 202611,
        ratio = 2f,
        isVideo = isVideo,
        durationMs = null,
        hasThumbnails = true,
        syncStatus = syncStatus,
    )

    private val photo = MediaDetails(cloudId = "c1", mimeType = "image/jpeg", width = 4000, height = 2000)

    @Test
    fun `exif orientation maps to its display rotation`() {
        assertEquals(0, MediaMutationPolicy.rotationDegrees(1))
        assertEquals(180, MediaMutationPolicy.rotationDegrees(3))
        assertEquals(90, MediaMutationPolicy.rotationDegrees(6))
        assertEquals(270, MediaMutationPolicy.rotationDegrees(8))
        // Mirrored orientations keep their rotation component.
        assertEquals(90, MediaMutationPolicy.rotationDegrees(5))
        assertEquals(270, MediaMutationPolicy.rotationDegrees(7))
    }

    @Test
    fun `rotation maps back to a non-mirrored exif orientation`() {
        assertEquals(1, MediaMutationPolicy.orientationForDegrees(0))
        assertEquals(6, MediaMutationPolicy.orientationForDegrees(90))
        assertEquals(3, MediaMutationPolicy.orientationForDegrees(180))
        assertEquals(8, MediaMutationPolicy.orientationForDegrees(270))
        assertEquals(6, MediaMutationPolicy.orientationForDegrees(450))
        assertEquals(8, MediaMutationPolicy.orientationForDegrees(-90))
    }

    @Test
    fun `8-entry table rotates standard and mirrored orientations like web`() {
        assertEquals(6, MediaMutationPolicy.rotatedOrientationByDegrees(1, 90))
        assertEquals(3, MediaMutationPolicy.rotatedOrientationByDegrees(6, 90))
        assertEquals(8, MediaMutationPolicy.rotatedOrientationByDegrees(6, 180))
        assertEquals(1, MediaMutationPolicy.rotatedOrientationByDegrees(6, 270))

        // Mirrored orientations preserve mirroring across clockwise turns.
        assertEquals(5, MediaMutationPolicy.rotatedOrientationByDegrees(2, 90))
        assertEquals(4, MediaMutationPolicy.rotatedOrientationByDegrees(5, 90))
        assertEquals(7, MediaMutationPolicy.rotatedOrientationByDegrees(4, 90))
        assertEquals(2, MediaMutationPolicy.rotatedOrientationByDegrees(7, 90))
    }

    @Test
    fun `ratio only flips for quarter turns`() {
        assertEquals(0.5f, MediaMutationPolicy.rotatedRatio(2f, baseDegrees = 0, targetDegrees = 90), 0.0001f)
        assertEquals(2f, MediaMutationPolicy.rotatedRatio(2f, baseDegrees = 0, targetDegrees = 180), 0.0001f)
        assertEquals(2f, MediaMutationPolicy.rotatedRatio(2f, baseDegrees = 0, targetDegrees = 0), 0.0001f)
        assertEquals(0.5f, MediaMutationPolicy.rotatedRatio(2f, baseDegrees = 270, targetDegrees = 0), 0.0001f)
        assertEquals(0.5f, MediaMutationPolicy.rotatedRatio(2f, netDegrees = 90), 0.0001f)
        assertEquals(2f, MediaMutationPolicy.rotatedRatio(2f, netDegrees = 180), 0.0001f)
    }

    @Test
    fun `optimistic rotation scales a quarter-turned photo back into the screen`() {
        // Portrait screen, landscape photo: rotating it still fits.
        assertEquals(
            1f,
            MediaMutationPolicy.optimisticRotationScale(1000f, 2000f, ratio = 2f, degrees = 90),
            0.0001f,
        )
        // Portrait photo on a portrait screen: the quarter turn needs a 0.5 scale.
        assertEquals(
            0.5f,
            MediaMutationPolicy.optimisticRotationScale(1000f, 2000f, ratio = 0.5f, degrees = 90),
            0.0001f,
        )
        assertEquals(
            1f,
            MediaMutationPolicy.optimisticRotationScale(1000f, 2000f, ratio = 0.5f, degrees = 180),
            0.0001f,
        )
    }

    @Test
    fun `rotate is offered only for still, decodable, non-panorama photos that are not missing`() {
        assertTrue(MediaMutationPolicy.canRotate(item(), photo, localInfo = null, isOnline = true))
        assertFalse(
            "offline",
            MediaMutationPolicy.canRotate(item(), photo, localInfo = null, isOnline = false),
        )
        assertFalse(
            "video",
            MediaMutationPolicy.canRotate(
                item(isVideo = true),
                photo.copy(isVideo = true),
                localInfo = null,
                isOnline = true,
            ),
        )
        assertFalse(
            "missing",
            MediaMutationPolicy.canRotate(
                item(),
                photo.copy(missingSince = "2026-01-01"),
                localInfo = null,
                isOnline = true,
            ),
        )
        assertFalse(
            "panorama",
            MediaMutationPolicy.canRotate(
                item(),
                photo.copy(usePanoramaViewer = true),
                localInfo = null,
                isOnline = true,
            ),
        )
        assertFalse(
            "tiff",
            MediaMutationPolicy.canRotate(item(), photo.copy(mimeType = "image/tiff"), localInfo = null, true),
        )
        assertFalse(
            "no mime",
            MediaMutationPolicy.canRotate(item(), photo.copy(mimeType = null), localInfo = null, true),
        )
        assertFalse(
            "cloud item still waiting for /item",
            MediaMutationPolicy.canRotate(item(), details = null, localInfo = null, isOnline = true),
        )
    }

    @Test
    fun `local-only photos rotate offline from their exif orientation`() {
        val local = item(
            id = "local_1",
            cloudId = null,
            localContentUri = "content://media/external/images/media/1",
            syncStatus = SyncStatus.LOCAL_ONLY,
        )
        val info = LocalMediaInfo(fileName = "local.jpg", mimeType = "image/jpeg", orientation = 6)
        assertTrue(MediaMutationPolicy.canRotate(local, details = null, localInfo = info, isOnline = false))
        assertEquals(90, MediaMutationPolicy.rotationDegrees(info.orientation))
    }

    @Test
    fun `reprocess needs a cloud item that still exists on the server`() {
        assertTrue(MediaMutationPolicy.canReprocess(item(), photo, isOnline = true))
        assertFalse("offline", MediaMutationPolicy.canReprocess(item(), photo, isOnline = false))
        assertFalse(
            "local only",
            MediaMutationPolicy.canReprocess(
                item(cloudId = null, localContentUri = "content://1", syncStatus = SyncStatus.LOCAL_ONLY),
                details = null,
                isOnline = true,
            ),
        )
        assertFalse(
            "missing on disk",
            MediaMutationPolicy.canReprocess(item(), photo.copy(missingSince = "2026-01-01"), true),
        )
    }
}
