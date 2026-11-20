package dev.ruurd.photos.domain.model

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PassOneDeduplicationTest {

    private val taken = 1_730_000_000_000L

    @Test
    fun `exact timestamp and same ratio match`() {
        assertTrue(PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken))
        assertTrue(PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + 1_500L))
    }

    @Test
    fun `different aspect ratio never matches`() {
        assertFalse(PassOneDeduplication.isMatch(1.5f, taken, 0.75f, taken))
    }

    @Test
    fun `whole and quarter hour timezone offsets match`() {
        for (offsetHours in listOf(1, 2, 5, 8, 12)) {
            assertTrue(
                "offset $offsetHours h",
                PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + offsetHours * 3_600_000L),
            )
        }
        assertTrue(
            "30 minute offset",
            PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + 5 * 3_600_000L + 1_800_000L),
        )
        assertTrue(
            "45 minute offset",
            PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + 5 * 3_600_000L + 2_700_000L),
        )
    }

    @Test
    fun `timestamp drift that is not a zone offset does not match`() {
        // One hour plus five minutes: a real edit of the capture time, not a timezone.
        assertFalse(PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + 3_900_000L))
        assertFalse(PassOneDeduplication.isMatch(1.5f, taken, 1.5f, taken + 25 * 3_600_000L))
    }
}
