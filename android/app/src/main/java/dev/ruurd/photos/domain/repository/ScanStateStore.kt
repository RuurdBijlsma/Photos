package dev.ruurd.photos.domain.repository

import kotlinx.coroutines.flow.StateFlow

/**
 * Remembers what the last completed `MediaStore` scan accounted for so an unchanged device can skip
 * the whole scan on cold start. Cleared together with the local rows on sign-out.
 */
interface ScanStateStore {
    val hasScanSignature: StateFlow<Boolean>
    suspend fun lastScanSignature(): String?
    suspend fun setLastScanSignature(signature: String)
}
