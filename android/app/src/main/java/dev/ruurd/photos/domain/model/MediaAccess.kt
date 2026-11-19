package dev.ruurd.photos.domain.model

/**
 * Per-type MediaStore access level. Android 14+ can grant a partial selection
 * (`READ_MEDIA_VISUAL_USER_SELECTED`) instead of full library access.
 */
enum class MediaAccess {
    NONE,
    PARTIAL,
    FULL,
}

data class MediaAccessState(
    val images: MediaAccess = MediaAccess.NONE,
    val videos: MediaAccess = MediaAccess.NONE,
) {
    val hasAnyAccess: Boolean
        get() = images != MediaAccess.NONE || videos != MediaAccess.NONE

    /** Partial access silently hides items, so cursor omission must never mean "deleted". */
    fun isFull(images: Boolean): Boolean = if (images) {
        this.images == MediaAccess.FULL
    } else {
        videos == MediaAccess.FULL
    }

    /** A partial grant only covers a picker selection, which the volume generation does not track. */
    val hasPartialAccess: Boolean
        get() = images == MediaAccess.PARTIAL || videos == MediaAccess.PARTIAL
}
