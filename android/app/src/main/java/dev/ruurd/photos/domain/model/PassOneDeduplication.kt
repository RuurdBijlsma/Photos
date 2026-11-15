package dev.ruurd.photos.domain.model

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
        localTimestampMs: Long
    ): Boolean {
        if (abs(cloudRatio - localRatio) >= RATIO_TOLERANCE) return false

        val diff = abs(cloudSortTimestamp - localTimestampMs)
        if (diff < TIMESTAMP_TOLERANCE_MS) return true
        if (diff > TIMEZONE_TOLERANCE_MS) return false

        val remainder = diff % OFFSET_STEP_MS
        return remainder < TIMESTAMP_TOLERANCE_MS || OFFSET_STEP_MS - remainder < TIMESTAMP_TOLERANCE_MS
    }
}
