package dev.ruurd.photos.core.permissions

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import dev.ruurd.photos.domain.model.MediaAccess
import dev.ruurd.photos.domain.model.MediaAccessState

/**
 * Reads the current MediaStore grant per media type. Android 14+ can hand out
 * `READ_MEDIA_VISUAL_USER_SELECTED` instead of the full library, which makes a short scan normal
 * rather than a sign of deleted files.
 */
object MediaPermissions {

    val requestedPermissions = arrayOf(
        Manifest.permission.READ_MEDIA_IMAGES,
        Manifest.permission.READ_MEDIA_VIDEO,
        Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
    )

    fun read(context: Context): MediaAccessState = MediaAccessState(
        images = accessFor(context, Manifest.permission.READ_MEDIA_IMAGES),
        videos = accessFor(context, Manifest.permission.READ_MEDIA_VIDEO),
    )

    private fun accessFor(context: Context, fullPermission: String): MediaAccess = when {
        isGranted(context, fullPermission) -> MediaAccess.FULL
        isGranted(context, Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED) -> MediaAccess.PARTIAL
        else -> MediaAccess.NONE
    }

    private fun isGranted(context: Context, permission: String): Boolean = ContextCompat.checkSelfPermission(context, permission) == PackageManager.PERMISSION_GRANTED
}
