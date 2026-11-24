
package dev.ruurd.photos.domain.model

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MediaFilePolicyTest {

    @Test
    fun `sanitize strips path separators and unsafe characters`() {
        assertEquals("evil.jpg", MediaFileNames.sanitize("../evil.jpg", isVideo = false, mimeType = null))
        assertEquals("a_b.c", MediaFileNames.sanitize("a b.c", isVideo = false, mimeType = null))
        assertEquals("photo_media.jpg", MediaFileNames.sanitize("", isVideo = false, mimeType = "image/jpeg"))
        assertEquals("clip.bin", MediaFileNames.sanitize("clip", isVideo = true, mimeType = null))
    }

    @Test
    fun `fallback names use the mime extension or a photo-video default`() {
        assertEquals("photo_c1.jpg", MediaFileNames.fallbackName("c1", isVideo = false, mimeType = "image/jpeg"))
        assertEquals("video_c1.mov", MediaFileNames.fallbackName("c1", isVideo = true, mimeType = "video/quicktime"))
        assertEquals("photo_c1.jpg", MediaFileNames.fallbackName("c1", isVideo = false, mimeType = null))
        assertEquals("video_c1.mp4", MediaFileNames.fallbackName("c1", isVideo = true, mimeType = null))
    }

    @Test
    fun `parses plain and rfc5987 content disposition filenames`() {
        assertEquals(
            "IMG_1234.jpg",
            MediaFileNames.parseContentDispositionFilename("""inline; filename="IMG_1234.jpg""""),
        )
        assertEquals(
            "vacation.png",
            MediaFileNames.parseContentDispositionFilename("attachment; filename=vacation.png"),
        )
        assertEquals(
            "my photo+1.heic",
            MediaFileNames.parseContentDispositionFilename(
                """inline; filename="fallback.jpg"; filename*=UTF-8''my%20photo+1.heic""",
            ),
        )
        assertNull(MediaFileNames.parseContentDispositionFilename(null))
        assertNull(MediaFileNames.parseContentDispositionFilename("inline"))
    }

    @Test
    fun `full res is fetched for decodable stills that still exist`() {
        assertTrue(FullResEligibility.shouldFetchFullRes(false, "image/jpeg", null))
        assertTrue(FullResEligibility.shouldFetchFullRes(false, "IMAGE/AVIF", null))
        assertFalse(FullResEligibility.shouldFetchFullRes(false, "image/tiff", null))
        assertFalse(FullResEligibility.shouldFetchFullRes(false, "image/jpeg", "2024-01-01"))
        assertFalse(FullResEligibility.shouldFetchFullRes(true, "image/jpeg", null))
        assertFalse(FullResEligibility.shouldFetchFullRes(false, null, null))
    }
}