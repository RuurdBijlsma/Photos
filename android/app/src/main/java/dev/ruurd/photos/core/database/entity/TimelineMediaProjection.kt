package dev.ruurd.photos.core.database.entity

import androidx.compose.runtime.Immutable
import androidx.room.ColumnInfo

@Immutable
data class TimelineMediaProjection(
    @ColumnInfo(name = "id")
    val id: String,

    @ColumnInfo(name = "cloud_id")
    val cloudId: String?,

    @ColumnInfo(name = "local_content_uri")
    val localContentUri: String?,

    @ColumnInfo(name = "year_month")
    val yearMonth: Int,

    @ColumnInfo(name = "ratio")
    val ratio: Float,

    @ColumnInfo(name = "is_video")
    val isVideo: Boolean,

    @ColumnInfo(name = "duration_ms")
    val durationMs: Int?,

    @ColumnInfo(name = "has_thumbnails")
    val hasThumbnails: Boolean,

    @ColumnInfo(name = "sync_status")
    val syncStatus: SyncStatus,

    @ColumnInfo(name = "sort_timestamp")
    val sortTimestamp: Long = 0L,

    @ColumnInfo(name = "timezone_offset_minutes")
    val timezoneOffsetMinutes: Int = 0
)