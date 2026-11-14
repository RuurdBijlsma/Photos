package dev.ruurd.photos.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "media_item",
    indices = [
        Index(value = ["local_content_uri"], unique = true),
        Index(value = ["cloud_id"], unique = false),
        Index(value = ["size_bytes"]),
        Index(value = ["bucket_id"]),
        Index(
            value = ["is_visible_in_timeline", "year_month", "sort_timestamp", "id"],
            orders = [Index.Order.ASC, Index.Order.DESC, Index.Order.DESC, Index.Order.DESC]
        )
    ]
)
data class MediaEntity(
    @PrimaryKey
    @ColumnInfo(name = "id")
    val id: String,

    @ColumnInfo(name = "cloud_id")
    val cloudId: String? = null,

    @ColumnInfo(name = "local_content_uri")
    val localContentUri: String? = null,

    @ColumnInfo(name = "bucket_id")
    val bucketId: String? = null,

    @ColumnInfo(name = "bucket_name")
    val bucketName: String? = null,

    @ColumnInfo(name = "relative_path")
    val relativePath: String? = null,

    @ColumnInfo(name = "is_visible_in_timeline")
    val isVisibleInTimeline: Boolean = true,

    @ColumnInfo(name = "auto_backup_suppressed")
    val autoBackupSuppressed: Boolean = false,

    @ColumnInfo(name = "size_bytes")
    val sizeBytes: Long = 0L,

    @ColumnInfo(name = "ratio")
    val ratio: Float = 1.0f,

    @ColumnInfo(name = "sort_timestamp")
    val sortTimestamp: Long = 0L,

    @ColumnInfo(name = "timezone_offset_minutes")
    val timezoneOffsetMinutes: Int = 0,

    @ColumnInfo(name = "year_month")
    val yearMonth: Int = 0,

    @ColumnInfo(name = "is_video")
    val isVideo: Boolean = false,

    @ColumnInfo(name = "duration_ms")
    val durationMs: Int? = null,

    @ColumnInfo(name = "has_thumbnails")
    val hasThumbnails: Boolean = false,

    @ColumnInfo(name = "blake3_hash")
    val blake3Hash: String? = null,

    @ColumnInfo(name = "sync_status")
    val syncStatus: SyncStatus = SyncStatus.LOCAL_ONLY
)
