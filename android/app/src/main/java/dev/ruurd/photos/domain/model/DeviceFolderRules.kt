package dev.ruurd.photos.domain.model

data class DeviceFolderDefaults(
    val showInTimeline: Boolean,
    val backupEnabled: Boolean,
)

/**
 * First-discovery defaults for a `MediaStore` bucket: camera folders under `DCIM/` are backed up
 * and shown in the timeline, everything else (Screenshots, Download, WhatsApp, ...) is opt-in.
 */
object DeviceFolderRules {

    private const val CAMERA_ROOT = "DCIM/"

    fun defaultsFor(relativePath: String?): DeviceFolderDefaults {
        val path = relativePath ?: ""
        val isCamera = path.startsWith(CAMERA_ROOT, ignoreCase = true) &&
            !path.startsWith("DCIM/Screenshots", ignoreCase = true) &&
            !path.startsWith("DCIM/Screen", ignoreCase = true)
        return DeviceFolderDefaults(showInTimeline = isCamera, backupEnabled = isCamera)
    }
}
