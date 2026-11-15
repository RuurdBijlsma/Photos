package dev.ruurd.photos.data.datasource.local

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.domain.model.LocalMediaFile
import java.time.Instant
import java.time.ZoneId

/** Maps a `MediaStore` row to a `media_item` row; pure so orientation/ratio/month math is testable. */
object LocalMediaMapper {

    fun ratio(width: Int, height: Int, orientation: Int): Float {
        // MediaStore reports raw sensor dimensions; the backend stores orientation-adjusted ones.
        val (w, h) = if (orientation == 90 || orientation == 270) height to width else width to height
        return if (w <= 0 || h <= 0) 1f else w.toFloat() / h.toFloat()
    }

    fun yearMonth(timestampMs: Long, zone: ZoneId): Int {
        val date = Instant.ofEpochMilli(timestampMs).atZone(zone)
        return date.year * 100 + date.monthValue
    }

    fun toEntity(
        file: LocalMediaFile,
        isVisibleInTimeline: Boolean,
        zone: ZoneId = ZoneId.systemDefault()
    ): MediaEntity = MediaEntity(
        id = rowId(file.mediaStoreId),
        cloudId = null,
        localContentUri = file.contentUri,
        bucketId = file.bucketId,
        bucketName = file.bucketName,
        relativePath = file.relativePath,
        isVisibleInTimeline = isVisibleInTimeline,
        autoBackupSuppressed = false,
        sizeBytes = file.sizeBytes,
        ratio = ratio(file.width, file.height, file.orientation),
        sortTimestamp = file.dateTakenMs,
        timezoneOffsetMinutes = 0,
        yearMonth = yearMonth(file.dateTakenMs, zone),
        isVideo = file.isVideo,
        durationMs = file.durationMs,
        hasThumbnails = false,
        blake3Hash = null,
        syncStatus = SyncStatus.LOCAL_ONLY
    )

    fun rowId(mediaStoreId: Long): String = "local_$mediaStoreId"
}
