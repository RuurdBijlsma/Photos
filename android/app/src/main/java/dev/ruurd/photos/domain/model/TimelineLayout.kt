package dev.ruurd.photos.domain.model

import androidx.compose.runtime.Immutable
import dev.ruurd.photos.core.database.entity.SyncStatus
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf

/**
 * One entry of the justified timeline: either a month header or a single precomputed row of photos.
 * All geometry is precomputed off the main thread, so composables never measure or format anything.
 */
@Immutable
sealed interface TimelineListItem {
    val id: String
    val yearMonth: Int
    val estimatedOffsetTopDp: Float
}

@Immutable
data class MonthHeaderUiItem(
    override val id: String,
    override val yearMonth: Int,
    /** Localized month name, e.g. "November". */
    val title: String,
    /** `null` for the current year, matching the web header. */
    val yearLabel: String?,
    override val estimatedOffsetTopDp: Float,
) : TimelineListItem

@Immutable
data class TimelineRowUiItem(
    override val id: String,
    override val yearMonth: Int,
    val heightDp: Float,
    val items: ImmutableList<TimelineGridItemUiModel>,
    val isFirstOfMonth: Boolean,
    val isLastOfMonth: Boolean,
    /** Thumbnail ladder step in px, picked from the rendered row height. */
    val thumbnailTargetHeightPx: Int,
    /** Pre-formatted capture date of the first item in this row (e.g. "Today", "Sat, 24 Oct"). */
    val dateLabel: String = "",
    override val estimatedOffsetTopDp: Float,
) : TimelineListItem

@Immutable
data class TimelineGridItemUiModel(
    val id: String,
    val localContentUri: String?,
    val cloudId: String?,
    val isVideo: Boolean,
    /** Pre-formatted duration for the video pill, e.g. "0:14". */
    val durationLabel: String?,
    val hasThumbnails: Boolean,
    /** Pixel-snapped tile width, so rows end flush against the right edge. */
    val widthDp: Float,
    val syncStatus: SyncStatus,
)

@Immutable
data class TimelineYearMark(val year: Int, val fraction: Float)

/** Binary-searchable index over the laid-out rows, used by the scrubber and scroll-to-item. */
@Immutable
class TimelineLayoutIndex(
    val rows: ImmutableList<TimelineListItem> = persistentListOf(),
    val totalHeightDp: Float = 0f,
    val headerHeightDp: Float = 0f,
    val monthOffsets: Map<Int, Int> = emptyMap(),
    val monthLabels: Map<Int, String> = emptyMap(),
    val itemOffsets: Map<String, Int> = emptyMap(),
    val yearFractions: ImmutableList<TimelineYearMark> = persistentListOf(),
) {

    val isEmpty: Boolean get() = rows.isEmpty()

    /** Row/header index whose top is nearest above [fraction] (0f..1f) of the total height. */
    fun findRowIndexAtPercentage(fraction: Float): Int = findRowIndexAtOffsetDp(fraction.coerceIn(0f, 1f) * totalHeightDp)

    /** Row/header index whose top is nearest above [offsetDp] from the top of the timeline. */
    fun findRowIndexAtOffsetDp(offsetDp: Float): Int {
        if (rows.isEmpty()) return 0
        val target = offsetDp.coerceIn(0f, totalHeightDp)
        var low = 0
        var high = rows.lastIndex
        var index = 0
        while (low <= high) {
            val mid = (low + high) / 2
            if (rows[mid].estimatedOffsetTopDp <= target) {
                index = mid
                low = mid + 1
            } else {
                high = mid - 1
            }
        }
        return index
    }

    fun findIndexForDate(yearMonth: Int): Int = monthOffsets[yearMonth] ?: 0

    fun findRowIndexForMedia(mediaId: String): Int? = itemOffsets[mediaId]

    fun monthLabelAt(index: Int): String? = rows.getOrNull(index)?.let { monthLabels[it.yearMonth] }

    fun yearAt(index: Int): Int? = rows.getOrNull(index)?.let { it.yearMonth / 100 }

    fun dateLabelAt(index: Int): String? {
        val row = when (val item = rows.getOrNull(index)) {
            is TimelineRowUiItem -> item
            is MonthHeaderUiItem -> rows.getOrNull(index + 1) as? TimelineRowUiItem
            null -> null
        }
        return row?.dateLabel?.takeIf { it.isNotEmpty() }
    }
}
