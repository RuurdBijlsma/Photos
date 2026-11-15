package dev.ruurd.photos.domain.model

/**
 * Conflicts between "the MediaStore cursor did not return this row" and "the file still exists"
 * are silent data loss, so deletion reconciliation is only allowed when the scan is trustworthy.
 */
object ScanGuards {

    /**
     * @param fullAccess full library permission for the queried media type (not partial selection)
     * @param scannedCount rows the cursor returned for this media type
     * @param localCount local rows of this media type currently in Room
     */
    fun shouldReconcileRemovals(fullAccess: Boolean, scannedCount: Int, localCount: Int): Boolean =
        fullAccess && !(scannedCount == 0 && localCount > 0)
}
