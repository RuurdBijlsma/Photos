package dev.ruurd.photos.domain.repository

/**
 * Remembers what the last completed `MediaStore` scan accounted for so an unchanged device can skip
 * the whole scan on cold start. Cleared together with the local rows on sign-out.
 */
interface ScanStateStore {
    suspend fun lastScanSignature(): String?
    suspend fun setLastScanSignature(signature: String)
}
