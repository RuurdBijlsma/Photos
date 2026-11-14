package dev.ruurd.photos.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "device_folder")
data class DeviceFolderEntity(
    @PrimaryKey
    @ColumnInfo(name = "bucket_id")
    val bucketId: String,

    @ColumnInfo(name = "display_name")
    val displayName: String,

    @ColumnInfo(name = "relative_path")
    val relativePath: String,

    @ColumnInfo(name = "show_in_timeline")
    val showInTimeline: Boolean = false,

    @ColumnInfo(name = "backup_enabled")
    val backupEnabled: Boolean = false
)
