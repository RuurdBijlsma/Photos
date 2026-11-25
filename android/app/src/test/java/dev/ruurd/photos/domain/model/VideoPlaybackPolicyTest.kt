package dev.ruurd.photos.domain.model

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VideoPlaybackPolicyTest {

    @Test
    fun `formatSpeed trims trailing zeroes for whole and fractional speeds`() {
        assertEquals("0.25x", VideoPlaybackPolicy.formatSpeed(0.25f))
        assertEquals("0.5x", VideoPlaybackPolicy.formatSpeed(0.5f))
        assertEquals("1x", VideoPlaybackPolicy.formatSpeed(1.0f))
        assertEquals("1.25x", VideoPlaybackPolicy.formatSpeed(1.25f))
        assertEquals("2x", VideoPlaybackPolicy.formatSpeed(2.0f))
    }

    @Test
    fun `formatVideoTime handles zero, minutes, and hours`() {
        assertEquals("0:00", VideoPlaybackPolicy.formatVideoTime(-500L))
        assertEquals("0:00", VideoPlaybackPolicy.formatVideoTime(0L))
        assertEquals("0:14", VideoPlaybackPolicy.formatVideoTime(14_000L))
        assertEquals("1:05", VideoPlaybackPolicy.formatVideoTime(65_000L))
        assertEquals("1:01:01", VideoPlaybackPolicy.formatVideoTime(3_661_000L))
    }

    @Test
    fun `resolveCloudQuality defaults based on thumbnails, network, and missing original`() {
        assertEquals(
            VideoQuality.ORIGINAL,
            VideoPlaybackPolicy.resolveCloudQuality(
                savedPreference = VideoQuality.P144,
                hasThumbnails = false,
                isOriginalMissing = false,
                isUnmeteredWithoutDataSaver = false,
                transientQualityOverride = VideoQuality.P480,
            ),
        )
        assertEquals(
            VideoQuality.ORIGINAL,
            VideoPlaybackPolicy.resolveCloudQuality(
                savedPreference = VideoQuality.AUTO,
                hasThumbnails = true,
                isOriginalMissing = false,
                isUnmeteredWithoutDataSaver = true,
                transientQualityOverride = null,
            ),
        )
        assertEquals(
            VideoQuality.P480,
            VideoPlaybackPolicy.resolveCloudQuality(
                savedPreference = VideoQuality.AUTO,
                hasThumbnails = true,
                isOriginalMissing = false,
                isUnmeteredWithoutDataSaver = false,
                transientQualityOverride = null,
            ),
        )
        assertEquals(
            VideoQuality.P480,
            VideoPlaybackPolicy.resolveCloudQuality(
                savedPreference = VideoQuality.ORIGINAL,
                hasThumbnails = true,
                isOriginalMissing = true,
                isUnmeteredWithoutDataSaver = true,
                transientQualityOverride = null,
            ),
        )
        assertEquals(
            VideoQuality.P480,
            VideoPlaybackPolicy.resolveCloudQuality(
                savedPreference = VideoQuality.ORIGINAL,
                hasThumbnails = true,
                isOriginalMissing = false,
                isUnmeteredWithoutDataSaver = true,
                transientQualityOverride = VideoQuality.P480,
            ),
        )
    }

    @Test
    fun `resolveSource prefers local unless failed, and builds expected cloud URLs`() {
        assertEquals(
            ResolvedVideoSource.Local("content://media/external/video/1"),
            VideoPlaybackPolicy.resolveSource(
                localContentUri = "content://media/external/video/1",
                localFailed = false,
                cloudId = "c1",
                baseUrl = "https://photos.example",
                hasThumbnails = true,
                isOriginalMissing = false,
                savedQuality = VideoQuality.AUTO,
                transientQualityOverride = null,
                isUnmeteredWithoutDataSaver = false,
            ),
        )

        assertEquals(
            ResolvedVideoSource.Cloud(
                url = "https://photos.example/thumbnails/c1/480p.webm",
                quality = VideoQuality.P480,
                requiresAuth = false,
            ),
            VideoPlaybackPolicy.resolveSource(
                localContentUri = "content://media/external/video/1",
                localFailed = true,
                cloudId = "c1",
                baseUrl = "https://photos.example",
                hasThumbnails = true,
                isOriginalMissing = false,
                savedQuality = VideoQuality.AUTO,
                transientQualityOverride = null,
                isUnmeteredWithoutDataSaver = false,
            ),
        )

        assertEquals(
            ResolvedVideoSource.Cloud(
                url = "https://photos.example/api/photos/c1/video",
                quality = VideoQuality.ORIGINAL,
                requiresAuth = true,
            ),
            VideoPlaybackPolicy.resolveSource(
                localContentUri = null,
                localFailed = false,
                cloudId = "c1",
                baseUrl = "https://photos.example",
                hasThumbnails = false,
                isOriginalMissing = false,
                savedQuality = VideoQuality.P144,
                transientQualityOverride = null,
                isUnmeteredWithoutDataSaver = false,
            ),
        )

        assertEquals(
            ResolvedVideoSource.Unavailable(VideoUnavailableReason.MISSING_SOURCE),
            VideoPlaybackPolicy.resolveSource(
                localContentUri = null,
                localFailed = false,
                cloudId = "c1",
                baseUrl = "https://photos.example",
                hasThumbnails = false,
                isOriginalMissing = true,
                savedQuality = VideoQuality.AUTO,
                transientQualityOverride = null,
                isUnmeteredWithoutDataSaver = true,
            ),
        )

        assertEquals(
            ResolvedVideoSource.Unavailable(VideoUnavailableReason.PLAYBACK_FAILED),
            VideoPlaybackPolicy.resolveSource(
                localContentUri = null,
                localFailed = false,
                cloudId = null,
                baseUrl = "https://photos.example",
                hasThumbnails = true,
                isOriginalMissing = false,
                savedQuality = VideoQuality.AUTO,
                transientQualityOverride = null,
                isUnmeteredWithoutDataSaver = true,
            ),
        )
    }

    @Test
    fun `availableQualities respects hasThumbnails, sourceHeight, and missing original`() {
        val noThumbs = VideoPlaybackPolicy.availableQualities(
            hasThumbnails = false,
            isOriginalMissing = false,
            sourceHeight = 2160,
        )
        assertEquals(
            listOf(VideoQualityOption(VideoQuality.ORIGINAL, "2160p (Original)", isEnabled = true)),
            noThumbs,
        )

        val withThumbsAndMissingOriginal = VideoPlaybackPolicy.availableQualities(
            hasThumbnails = true,
            isOriginalMissing = true,
            sourceHeight = null,
        )
        assertEquals(
            listOf(
                VideoQualityOption(VideoQuality.ORIGINAL, "Original", isEnabled = false),
                VideoQualityOption(VideoQuality.P480, "480p", isEnabled = true),
                VideoQualityOption(VideoQuality.P144, "144p", isEnabled = true),
            ),
            withThumbsAndMissingOriginal,
        )
    }

    @Test
    fun `shouldFallbackTo480p only falls back from original with thumbnails on codec or 404-415-500`() {
        assertTrue(
            VideoPlaybackPolicy.shouldFallbackTo480p(
                hasThumbnails = true,
                currentQuality = VideoQuality.ORIGINAL,
                isCodecOrParsingError = true,
                isBadHttpStatus = false,
                httpStatusCode = null,
            ),
        )
        for (code in listOf(404, 415, 500)) {
            assertTrue(
                VideoPlaybackPolicy.shouldFallbackTo480p(
                    hasThumbnails = true,
                    currentQuality = VideoQuality.ORIGINAL,
                    isCodecOrParsingError = false,
                    isBadHttpStatus = true,
                    httpStatusCode = code,
                ),
            )
        }
        for (code in listOf(401, 403, 503)) {
            assertFalse(
                VideoPlaybackPolicy.shouldFallbackTo480p(
                    hasThumbnails = true,
                    currentQuality = VideoQuality.ORIGINAL,
                    isCodecOrParsingError = false,
                    isBadHttpStatus = true,
                    httpStatusCode = code,
                ),
            )
        }
        assertFalse(
            VideoPlaybackPolicy.shouldFallbackTo480p(
                hasThumbnails = false,
                currentQuality = VideoQuality.ORIGINAL,
                isCodecOrParsingError = true,
                isBadHttpStatus = false,
                httpStatusCode = null,
            ),
        )
        assertFalse(
            VideoPlaybackPolicy.shouldFallbackTo480p(
                hasThumbnails = true,
                currentQuality = VideoQuality.P480,
                isCodecOrParsingError = true,
                isBadHttpStatus = false,
                httpStatusCode = null,
            ),
        )
    }
}