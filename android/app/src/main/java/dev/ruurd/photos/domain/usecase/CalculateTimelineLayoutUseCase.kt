package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.MonthHeaderUiItem
import dev.ruurd.photos.domain.model.TimelineGridItemUiModel
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.domain.model.TimelineListItem
import dev.ruurd.photos.domain.model.TimelineRowUiItem
import dev.ruurd.photos.domain.model.TimelineYearMark
import kotlinx.collections.immutable.toPersistentList
import java.time.Instant
import java.time.LocalDate
import java.time.Month
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.TextStyle
import java.util.Locale
import kotlin.math.min
import kotlin.math.roundToInt

/** Everything that can change the justified row geometry, in raw pixels so snapping is exact. */
data class TimelineLayoutParams(
    val containerWidthPx: Int,
    val density: Float,
    val desiredRowHeightDp: Float = DEFAULT_ROW_HEIGHT_DP
) {
    companion object {
        const val DEFAULT_ROW_HEIGHT_DP = 180f
    }
}

/**
 * Partitions the ordered timeline into justified horizontal rows (the same linear partition the web
 * timeline uses): accumulate items until a row would exceed the container width, then scale that row
 * down so it ends flush. Runs on `Dispatchers.Default`; the result is fully precomputed for Compose.
 */
class CalculateTimelineLayoutUseCase(
    private val maxThumbnailHeightPx: Int = DEFAULT_MAX_THUMBNAIL_PX,
    private val currentYearProvider: () -> Int = { LocalDate.now().year },
    private val todayProvider: () -> LocalDate = { LocalDate.now() }
) {

    operator fun invoke(
        media: List<TimelineMediaProjection>,
        params: TimelineLayoutParams
    ): TimelineLayoutIndex {
        if (media.isEmpty() || params.containerWidthPx <= 0) return TimelineLayoutIndex()

        val density = params.density.takeIf { it > 0f } ?: 1f
        val containerWidthPx = params.containerWidthPx
        val gapPx = (ITEM_GAP_DP * density).roundToInt()
        val headerHeightPx = (HEADER_HEIGHT_DP * density).roundToInt()
        val desiredHeightPx = (params.desiredRowHeightDp * density).roundToInt().coerceAtLeast(1)
        val currentYear = currentYearProvider()
        val today = todayProvider()

        val items = ArrayList<TimelineListItem>(media.size + 16)
        val monthOffsets = HashMap<Int, Int>()
        val monthLabels = HashMap<Int, String>()
        val itemOffsets = HashMap<String, Int>(media.size)
        val yearStarts = LinkedHashMap<Int, Int>()
        var offsetPx = 0

        val rowMedia = ArrayList<TimelineMediaProjection>()
        var rowWidthPx = 0f

        // Consecutive rows are almost always on the same day, so both the label and the date are
        // computed once per day instead of once per row.
        var lastDateKey = Long.MIN_VALUE
        var lastDateLabel = ""

        fun dateLabelOf(item: TimelineMediaProjection): String {
            if (item.sortTimestamp <= 0L) return ""
            val key = (item.sortTimestamp + item.timezoneOffsetMinutes * 60_000L).floorDiv(DAY_MS)
            if (key != lastDateKey) {
                lastDateKey = key
                lastDateLabel = formatDateLabel(item, today, currentYear)
            }
            return lastDateLabel
        }

        fun finishRow(firstOfMonth: Boolean, lastOfMonth: Boolean, flushToWidth: Boolean) {
            if (rowMedia.isEmpty()) return
            val gapsPx = (rowMedia.size - 1) * gapPx
            val availableWidth = (containerWidthPx - gapsPx).coerceAtLeast(1).toFloat()
            val rawMultiplier = availableWidth / rowWidthPx
            val isFlush = flushToWidth || rawMultiplier <= MAX_SIZE_MULTIPLIER
            val multiplier = when {
                flushToWidth -> min(rawMultiplier, MAX_SIZE_MULTIPLIER)
                rawMultiplier <= MAX_SIZE_MULTIPLIER -> rawMultiplier
                // A month that ends on an incomplete row must not stretch into giant blocks.
                else -> 1f
            }
            val heightPx = (desiredHeightPx * multiplier).roundToInt().coerceAtLeast(1)
            val widths = IntArray(rowMedia.size) { index ->
                (ratioOf(rowMedia[index]) * heightPx).roundToInt().coerceAtLeast(1)
            }
            if (isFlush) {
                // Rounding leaves a few px; hand them to the last tile so the row ends flush.
                val leftover = containerWidthPx - widths.sum() - gapsPx
                val last = widths.lastIndex
                if (leftover != 0 && widths[last] + leftover > 0) widths[last] += leftover
            }

            val firstItem = rowMedia.first()
            val uiItems = rowMedia.mapIndexed { index, item ->
                item.toGridItem(widthDp = widths[index] / density)
            }.toPersistentList()

            rowMedia.forEach { item -> itemOffsets[item.id] = items.size }
            items.add(
                TimelineRowUiItem(
                    id = "row_${firstItem.id}",
                    yearMonth = firstItem.yearMonth,
                    heightDp = heightPx / density,
                    items = uiItems,
                    isFirstOfMonth = firstOfMonth,
                    isLastOfMonth = lastOfMonth,
                    thumbnailTargetHeightPx = thumbnailHeightFor(heightPx),
                    dateLabel = dateLabelOf(firstItem),
                    estimatedOffsetTopDp = offsetPx / density
                )
            )

            offsetPx += heightPx
            if (!lastOfMonth) offsetPx += gapPx
            rowMedia.clear()
            rowWidthPx = 0f
        }

        // The query is ordered by year_month DESC, so months are consecutive runs; grouping with
        // `groupBy` would allocate a map plus a list per month for the whole library.
        var monthStart = 0
        while (monthStart < media.size) {
            val yearMonth = media[monthStart].yearMonth
            var monthEnd = monthStart + 1
            while (monthEnd < media.size && media[monthEnd].yearMonth == yearMonth) monthEnd++

            yearStarts.putIfAbsent(yearMonth / 100, offsetPx)
            monthOffsets[yearMonth] = items.size
            monthLabels[yearMonth] = shortMonthLabel(yearMonth)
            items.add(
                MonthHeaderUiItem(
                    id = "header_$yearMonth",
                    yearMonth = yearMonth,
                    title = monthTitle(yearMonth),
                    yearLabel = (yearMonth / 100).takeIf { it != currentYear }?.toString(),
                    estimatedOffsetTopDp = offsetPx / density
                )
            )
            offsetPx += headerHeightPx

            var firstOfMonth = true
            for (index in monthStart until monthEnd) {
                val item = media[index]
                rowMedia.add(item)
                rowWidthPx += ratioOf(item) * desiredHeightPx
                val gapsPx = (rowMedia.size - 1) * gapPx
                if (rowWidthPx + gapsPx > containerWidthPx) {
                    finishRow(
                        firstOfMonth = firstOfMonth,
                        lastOfMonth = index == monthEnd - 1,
                        flushToWidth = true
                    )
                    firstOfMonth = false
                }
            }
            finishRow(firstOfMonth = firstOfMonth, lastOfMonth = true, flushToWidth = false)
            monthStart = monthEnd
        }

        val totalHeightPx = offsetPx
        val yearFractions = yearStarts.map { (year, startPx) ->
            TimelineYearMark(year, if (totalHeightPx > 0) startPx.toFloat() / totalHeightPx else 0f)
        }.toPersistentList()

        return TimelineLayoutIndex(
            rows = items.toPersistentList(),
            totalHeightDp = totalHeightPx / density,
            headerHeightDp = HEADER_HEIGHT_DP,
            monthOffsets = monthOffsets,
            monthLabels = monthLabels,
            itemOffsets = itemOffsets,
            yearFractions = yearFractions
        )
    }

    /** Smallest ladder step that is still larger than the rendered row height, capped for decode cost. */
    private fun thumbnailHeightFor(rowHeightPx: Int): Int {
        val target = min(rowHeightPx, maxThumbnailHeightPx)
        return THUMBNAIL_LADDER_PX.firstOrNull { it > target }?.coerceAtMost(maxThumbnailHeightPx)
            ?: maxThumbnailHeightPx
    }

    private fun TimelineMediaProjection.toGridItem(widthDp: Float) = TimelineGridItemUiModel(
        id = id,
        localContentUri = localContentUri,
        cloudId = cloudId,
        isVideo = isVideo,
        durationLabel = if (isVideo) durationMs?.let(::formatDuration) else null,
        hasThumbnails = hasThumbnails,
        widthDp = widthDp,
        syncStatus = syncStatus
    )

    private fun formatDateLabel(
        item: TimelineMediaProjection,
        today: LocalDate,
        currentYear: Int
    ): String {
        if (item.sortTimestamp <= 0L) return ""
        val localDate = Instant.ofEpochMilli(
            item.sortTimestamp + item.timezoneOffsetMinutes * 60_000L
        ).atZone(ZoneOffset.UTC).toLocalDate()

        return when {
            localDate == today -> "Today"
            localDate == today.minusDays(1) -> "Yesterday"
            localDate.year == currentYear -> localDate.format(CURRENT_YEAR_DATE_FORMATTER)
            else -> localDate.format(OTHER_YEAR_DATE_FORMATTER)
        }
    }

    private fun formatDuration(durationMs: Int): String {
        val totalSeconds = durationMs / 1000
        val minutes = totalSeconds / 60
        val seconds = totalSeconds % 60
        return if (seconds < 10) "$minutes:0$seconds" else "$minutes:$seconds"
    }

    private fun monthTitle(yearMonth: Int): String =
        monthOf(yearMonth)?.getDisplayName(TextStyle.FULL, Locale.getDefault()).orEmpty()

    private fun shortMonthLabel(yearMonth: Int): String {
        val month = monthOf(yearMonth) ?: return ""
        return "${month.getDisplayName(TextStyle.SHORT, Locale.getDefault())} ${yearMonth / 100}"
    }

    private fun monthOf(yearMonth: Int): Month? {
        val month = yearMonth % 100
        return if (yearMonth / 100 > 0 && month in 1..12) Month.of(month) else null
    }

    companion object {
        const val ITEM_GAP_DP = 2f
        const val HEADER_HEIGHT_DP = 64f
        const val MAX_SIZE_MULTIPLIER = 1.5f
        const val DEFAULT_MAX_THUMBNAIL_PX = 720
        val THUMBNAIL_LADDER_PX = intArrayOf(144, 240, 360, 480, 720, 1080, 1440)

        private val CURRENT_YEAR_DATE_FORMATTER =
            DateTimeFormatter.ofPattern("EEE, d MMM", Locale.ENGLISH)
        private val OTHER_YEAR_DATE_FORMATTER =
            DateTimeFormatter.ofPattern("EEE, d MMM yyyy", Locale.ENGLISH)

        private const val DAY_MS = 86_400_000L

        fun ratioOf(item: TimelineMediaProjection): Float =
            if (item.ratio.isFinite() && item.ratio > 0f) item.ratio else 1f
    }
}