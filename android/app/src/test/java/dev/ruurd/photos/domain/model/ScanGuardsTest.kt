package dev.ruurd.photos.domain.model

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ScanGuardsTest {

    @Test
    fun `full access with matching rows reconciles removals`() {
        assertTrue(ScanGuards.shouldReconcileRemovals(fullAccess = true, scannedCount = 3, localCount = 3))
        assertTrue(ScanGuards.shouldReconcileRemovals(fullAccess = true, scannedCount = 2, localCount = 3))
    }

    @Test
    fun `partial access never treats omission as deletion`() {
        assertFalse(ScanGuards.shouldReconcileRemovals(fullAccess = false, scannedCount = 1, localCount = 500))
    }

    @Test
    fun `an empty cursor is not trusted to delete everything`() {
        assertFalse(ScanGuards.shouldReconcileRemovals(fullAccess = true, scannedCount = 0, localCount = 40))
        // Nothing to delete when Room is empty too.
        assertTrue(ScanGuards.shouldReconcileRemovals(fullAccess = true, scannedCount = 0, localCount = 0))
    }
}
