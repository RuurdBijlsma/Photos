package dev.ruurd.photos.domain.model

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import kotlin.math.abs

/**
 * Pass 1 local/cloud deduplication: no hashing, just byte size plus aspect ratio and a timestamp
 * that either matches exactly or is off by a full 15-minute zone boundary (all real UTC offsets
 * are multiples of 15 minutes).
 */
object PassOneDeduplication {

    private const val TIMESTAMP_TOLERANCE_MS = 2_000L
    private const val TIMEZONE_TOLERANCE_MS = 24 * 3_600_000L
    private const val OFFSET_STEP_MS = 900_000L
    private const val RATIO_TOLERANCE = 0.02f

    fun isMatch(
        cloudRatio: Float,
        cloudSortTimestamp: Long,
        localRatio: Float,
        localTimestampMs: Long,
    ): Boolean {
        if (abs(cloudRatio - localRatio) >= RATIO_TOLERANCE) return false

        val diff = abs(cloudSortTimestamp - localTimestampMs)
        if (diff < TIMESTAMP_TOLERANCE_MS) return true
        if (diff > TIMEZONE_TOLERANCE_MS) return false

        val remainder = diff % OFFSET_STEP_MS
        return remainder < TIMESTAMP_TOLERANCE_MS || OFFSET_STEP_MS - remainder < TIMESTAMP_TOLERANCE_MS
    }

    /** First unlinked [candidates] row matching [ratio] and [sortTimestamp], skipping [excludeIds]. */
    fun findMatch(
        candidates: Iterable<MediaEntity>,
        excludeIds: Set<String>,
        ratio: Float,
        sortTimestamp: Long,
    ): MediaEntity? = candidates.firstOrNull { candidate ->
        candidate.id !in excludeIds &&
                isMatch(
                    cloudRatio = ratio,
                    cloudSortTimestamp = sortTimestamp,
                    localRatio = candidate.ratio,
                    localTimestampMs = candidate.sortTimestamp,
                )
    }

    /**
     * Merges a local file row with its matching cloud item into one canonical `SYNCED` row,
     * keeping the local URI/folder details and the server's authoritative media metadata.
     */
    fun mergeSynced(
        local: MediaEntity,
        cloud: MediaEntity,
        id: String,
    ): MediaEntity = cloud.copy(
        id = id,
        localContentUri = local.localContentUri,
        bucketId = local.bucketId,
        bucketName = local.bucketName,
        relativePath = local.relativePath,
        blake3Hash = local.blake3Hash,
        isVisibleInTimeline = true,
        autoBackupSuppressed = false,
        syncStatus = SyncStatus.SYNCED,
    )
}