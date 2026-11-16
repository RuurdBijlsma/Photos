package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.MonthHeaderUiItem
import dev.ruurd.photos.domain.model.TimelineRowUiItem
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.roundToInt

class CalculateTimelineLayoutUseCaseTest {

    private val density = 3f
    private val containerWidthPx = 1080
    private val params = TimelineLayoutParams(containerWidthPx = containerWidthPx, density = density)
    private val useCase = CalculateTimelineLayoutUseCase(currentYearProvider = { 2026 })

    private fun media(id: String, yearMonth: Int, ratio: Float = 1f) = TimelineMediaProjection(
        id = id,
        cloudId = null,
        localContentUri = null,
        yearMonth = yearMonth,
        ratio = ratio,
        isVideo = false,
        durationMs = null,
        hasThumbnails = false,
        syncStatus = SyncStatus.LOCAL_ONLY
    )

    /** Width/height ratio of square items, so a full row is exactly two tiles. */
    private fun square(id: String, yearMonth: Int) = media(id, yearMonth, 1f)

    @Test
    fun `month header precedes rows and offsets accumulate`() {
        val layout = useCase(listOf(square("a", 202610), square("b", 202610)), params)

        assertEquals(2, layout.rows.size)
        assertTrue(layout.rows[0] is MonthHeaderUiItem)
        assertTrue(layout.rows[1] is TimelineRowUiItem)
        assertEquals(0f, layout.rows[0].estimatedOffsetTopDp, 0.01f)
        assertEquals(layout.headerHeightDp, layout.rows[1].estimatedOffsetTopDp, 0.01f)

        val row = layout.rows[1] as TimelineRowUiItem
        assertEquals(
            row.estimatedOffsetTopDp + row.heightDp,
            layout.totalHeightDp,
            0.01f
        )
    }

    @Test
    fun `full rows end flush against the container edge`() {
        val layout = useCase((1..10).map { square("square$it", 202610) }, params)
        val rows = layout.rows.filterIsInstance<TimelineRowUiItem>()

        assertEquals(5, rows.size)
        val gapPx = (CalculateTimelineLayoutUseCase.ITEM_GAP_DP * density).roundToInt()
        rows.forEach { row ->
            val tilesPx = row.items.sumOf { (it.widthDp * density).roundToInt() }
            assertEquals(containerWidthPx, tilesPx + (row.items.size - 1) * gapPx)
            assertTrue(row.isFirstOfMonth == (row == rows.first()))
        }
        assertTrue(rows.last().isLastOfMonth)
    }

    @Test
    fun `trailing row within stretch limit fits container width without gap overflow`() {
        // Two 3:4 portrait items (405 px each at 180.dp * 3 = 540 px) total 816 px with gap (< 1080),
        // so they land in the trailing finishRow call with rawMultiplier ~ 1.33 <= 1.5.
        val layout = useCase(
            listOf(media("p1", 202610, 0.75f), media("p2", 202610, 0.75f)),
            params
        )
        val row = layout.rows.filterIsInstance<TimelineRowUiItem>().single()
        val gapPx = (CalculateTimelineLayoutUseCase.ITEM_GAP_DP * density).roundToInt()
        val tilesPx = row.items.sumOf { (it.widthDp * density).roundToInt() }

        assertEquals(containerWidthPx, tilesPx + gapPx)
    }

    @Test
    fun `incomplete month row does not stretch beyond the desired height`() {
        // Three squares: a full row of two, then a single tile that must stay at the desired height.
        val layout = useCase((1..3).map { square("square$it", 202610) }, params)
        val rows = layout.rows.filterIsInstance<TimelineRowUiItem>()

        assertEquals(2, rows.size)
        assertEquals(params.desiredRowHeightDp, rows.last().heightDp, 0.01f)
    }

    @Test
    fun `thumbnail ladder follows rendered pixels and honours the cap`() {
        val layout = useCase((1..2).map { square("square$it", 202610) }, params)
        val row = layout.rows.filterIsInstance<TimelineRowUiItem>().single()
        // A 180.dp row is ~537 px at 3x, which needs the 720 ladder step.
        assertEquals(720, row.thumbnailTargetHeightPx)

        val lowRam = CalculateTimelineLayoutUseCase(
            maxThumbnailHeightPx = 480,
            currentYearProvider = { 2026 }
        )
        val capped = lowRam((1..2).map { square("square$it", 202610) }, params)
        assertEquals(
            480,
            capped.rows.filterIsInstance<TimelineRowUiItem>().single().thumbnailTargetHeightPx
        )
    }

    @Test
    fun `layout index resolves media, months and percentages`() {
        val layout = useCase(
            listOf(square("new1", 202610), square("new2", 202610), square("old1", 202506)),
            params
        )

        val rowForNew2 = layout.findRowIndexForMedia("new2")
        assertNotNull(rowForNew2)
        assertTrue(layout.rows[rowForNew2!!] is TimelineRowUiItem)

        val oldMonthIndex = layout.findIndexForDate(202506)
        assertTrue(layout.rows[oldMonthIndex] is MonthHeaderUiItem)

        assertEquals(0, layout.findRowIndexAtPercentage(0f))
        assertEquals(layout.rows.lastIndex, layout.findRowIndexAtPercentage(1f))
        assertTrue(layout.findRowIndexAtPercentage(0.5f) in 0..layout.rows.lastIndex)
        assertEquals(0, layout.findRowIndexAtOffsetDp(0f))
        assertEquals(layout.rows.lastIndex, layout.findRowIndexAtOffsetDp(layout.totalHeightDp))
        assertTrue(layout.yearFractions.isNotEmpty())
        assertEquals(2026, layout.yearAt(0))
    }

    @Test
    fun `empty input and zero width produce an empty layout`() {
        assertTrue(useCase(emptyList(), params).isEmpty)
        assertTrue(useCase(listOf(square("a", 202610)), params.copy(containerWidthPx = 0)).isEmpty)
    }
}