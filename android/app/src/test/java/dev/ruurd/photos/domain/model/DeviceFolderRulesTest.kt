package dev.ruurd.photos.domain.model

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceFolderRulesTest {

    @Test
    fun `camera folders under DCIM are shown and backed up`() {
        for (path in listOf("DCIM/Camera/", "DCIM/OpenCamera/", "dcim/camera/")) {
            val defaults = DeviceFolderRules.defaultsFor(path)
            assertTrue(path, defaults.showInTimeline)
            assertTrue(path, defaults.backupEnabled)
        }
    }

    @Test
    fun `screenshots and screen recordings stay out of the timeline`() {
        for (path in listOf("DCIM/Screenshots/", "DCIM/Screen recordings/", "Pictures/Screenshots/")) {
            val defaults = DeviceFolderRules.defaultsFor(path)
            assertFalse(path, defaults.showInTimeline)
            assertFalse(path, defaults.backupEnabled)
        }
    }

    @Test
    fun `other device folders are opt-in`() {
        for (path in listOf("Download/", "Pictures/WhatsApp/", "Movies/", null)) {
            val defaults = DeviceFolderRules.defaultsFor(path)
            assertFalse(defaults.showInTimeline)
            assertFalse(defaults.backupEnabled)
        }
    }
}
