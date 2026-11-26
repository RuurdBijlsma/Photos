package dev.ruurd.photos.domain.model

import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import java.util.Locale
import kotlin.math.min

/**
 * EXIF-orientation math, rotate/reprocess eligibility, and the optimistic preview scale used while a
 * rotation commit is still debouncing. Uses the same 8-entry orientation lookup table as web.
 */
object MediaMutationPolicy {

    /** MIME types the phone can decode, write and re-read locally (TIFF/TGA stay server-only). */
    private val ROTATABLE_MIME_TYPES = setOf("image/jpeg", "image/jpg", "image/png", "image/webp")

    /** Rotation each EXIF orientation needs to display upright, indexed by `orientation - 1`. */
    private val EXIF_ROTATION_DEGREES = intArrayOf(0, 0, 180, 180, 90, 90, 270, 270)

    /** Web's 8-entry EXIF orientation lookup table for 0°/90°/180°/270° clockwise turns. */
    private val ROTATION_TABLE = arrayOf(
        intArrayOf(1, 6, 3, 8), // 1
        intArrayOf(2, 5, 4, 7), // 2
        intArrayOf(3, 8, 1, 6), // 3
        intArrayOf(4, 7, 2, 5), // 4
        intArrayOf(5, 4, 7, 2), // 5
        intArrayOf(6, 3, 8, 1), // 6
        intArrayOf(7, 2, 5, 4), // 7
        intArrayOf(8, 1, 6, 3), // 8
    )

    fun rotationDegrees(orientation: Int): Int = EXIF_ROTATION_DEGREES[(orientation - 1).coerceIn(0, 7)]

    /** The non-mirrored orientation for a clockwise rotation from upright. */
    fun orientationForDegrees(degrees: Int): Int = when (((degrees % 360) + 360) % 360) {
        90 -> 6
        180 -> 3
        270 -> 8
        else -> 1
    }

    /** Applies [degrees] clockwise rotation to [orientation], preserving mirror state when present. */
    fun rotatedOrientationByDegrees(orientation: Int, degrees: Int): Int {
        val row = ROTATION_TABLE[(orientation - 1).coerceIn(0, 7)]
        val quarterTurns = (((degrees / 90) % 4) + 4) % 4
        return row[quarterTurns]
    }

    fun rotatedRatio(ratio: Float, netDegrees: Int): Float = if (((netDegrees % 180) + 180) % 180 != 0) 1f / ratio else ratio

    fun rotatedRatio(ratio: Float, baseDegrees: Int, targetDegrees: Int): Float = if (isQuarterTurn(baseDegrees, targetDegrees)) 1f / ratio else ratio

    fun isQuarterTurn(baseDegrees: Int, targetDegrees: Int): Boolean = (((targetDegrees - baseDegrees) % 180) + 180) % 180 != 0

    /**
     * Scale that keeps a photo inside the screen while its preview is optimistically rotated by
     * [degrees]: the fitted box rotates too, so a quarter turn swaps its width and height.
     */
    fun optimisticRotationScale(
        containerWidth: Float,
        containerHeight: Float,
        ratio: Float,
        degrees: Int,
    ): Float {
        if (degrees % 180 == 0) return 1f
        if (containerWidth <= 0f || containerHeight <= 0f || ratio <= 0f) return 1f
        val fittedWidth: Float
        val fittedHeight: Float
        if (ratio >= containerWidth / containerHeight) {
            fittedWidth = containerWidth
            fittedHeight = containerWidth / ratio
        } else {
            fittedHeight = containerHeight
            fittedWidth = containerHeight * ratio
        }
        return min(1f, min(containerWidth / fittedHeight, containerHeight / fittedWidth))
    }

    /**
     * Cloud and synced items wait for `/item` so the current `orientation` is known; local-only
     * photos fall back to the EXIF orientation read with the info sheet and never need the network.
     */
    fun canRotate(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
        isOnline: Boolean,
    ): Boolean {
        if (item.isVideo) return false
        if (details?.isMissing == true || details?.usePanoramaViewer == true) return false
        val mimeType = (details?.mimeType ?: localInfo?.mimeType)?.lowercase(Locale.US)
        if (mimeType !in ROTATABLE_MIME_TYPES) return false
        // Synced and cloud items need the server's current orientation before the first tap.
        if (item.cloudId != null && (details == null || !isOnline)) return false
        return true
    }

    fun canReprocess(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        isOnline: Boolean,
    ): Boolean {
        if (item.cloudId == null || !isOnline) return false
        return details?.isMissing != true
    }

    /** EXIF orientation (1–8) the optimistic rotation starts from. */
    fun baseOrientation(details: MediaDetails?, localInfo: LocalMediaInfo?): Int = details?.orientation ?: localInfo?.orientation ?: 1

    /** Orientation degrees the optimistic rotation starts from. */
    fun baseRotationDegrees(details: MediaDetails?, localInfo: LocalMediaInfo?): Int = rotationDegrees(baseOrientation(details, localInfo))
}
