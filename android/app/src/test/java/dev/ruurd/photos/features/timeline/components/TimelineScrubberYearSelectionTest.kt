package dev.ruurd.photos.features.timeline.components

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TimelineScrubberYearSelectionTest {

    @Test
    fun `prioritizes keeping the oldest year mark when intermediate marks crowd it`() {
        // Years: 2026 (top), 2015, 2005, 2004, 2000, 1997 (bottom, oldest)
        val marks = listOf(
            PlacedYearMark(top = 0f, height = 20f), // 0: 2026
            PlacedYearMark(top = 100f, height = 20f), // 1: 2015
            PlacedYearMark(top = 472f, height = 20f), // 2: 2005 (collides with 1997 below)
            PlacedYearMark(top = 478f, height = 20f), // 3: 2004
            PlacedYearMark(top = 482f, height = 20f), // 4: 2000
            PlacedYearMark(top = 485f, height = 20f), // 5: 1997 (oldest)
        )

        val selected = selectVisibleYearMarkIndices(marks, gap = 4f)

        assertEquals(listOf(0, 1, 5), selected)
    }

    @Test
    fun `includes intermediate year mark when it leaves enough gap above oldest year`() {
        val marks = listOf(
            PlacedYearMark(top = 0f, height = 20f), // 0: 2026
            PlacedYearMark(top = 455f, height = 20f), // 1: 2005 (ends at 475 <= 485 - 4)
            PlacedYearMark(top = 480f, height = 20f), // 2: 2000 (overlaps 2005 and 1997)
            PlacedYearMark(top = 485f, height = 20f), // 3: 1997
        )

        val selected = selectVisibleYearMarkIndices(marks, gap = 4f)

        assertEquals(listOf(0, 1, 3), selected)
    }

    @Test
    fun `handles single and overlapping two-mark ranges safely`() {
        assertTrue(selectVisibleYearMarkIndices(emptyList(), gap = 4f).isEmpty())
        assertEquals(listOf(0), selectVisibleYearMarkIndices(listOf(PlacedYearMark(0f, 20f)), gap = 4f))
        assertEquals(
            listOf(0),
            selectVisibleYearMarkIndices(
                listOf(PlacedYearMark(0f, 20f), PlacedYearMark(10f, 20f)),
                gap = 4f,
            ),
        )
    }
}
