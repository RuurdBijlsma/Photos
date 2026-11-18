package dev.ruurd.photos.core.navigation

import kotlinx.serialization.Serializable

@Serializable
data object MainRoute

@Serializable
data class DeviceFoldersRoute(val bucketId: String? = null)

@Serializable
data class AlbumDetailRoute(val albumId: String)

@Serializable
data object BackupSettingsRoute

/**
 * Generic source contract for the media viewer, starting with Timeline in Phase 2
 * and extensible to DeviceFolder in Phase 3 and Album in Phase 5.
 */
@Serializable
sealed interface ViewerSource {
    @Serializable
    data object Timeline : ViewerSource

    @Serializable
    data class DeviceFolder(val bucketId: String) : ViewerSource

    @Serializable
    data class Album(val albumId: String) : ViewerSource
}

/**
 * Arguments needed to open the media viewer overlay.
 */
data class ViewerArgs(
    val initialMediaId: String,
    val source: ViewerSource = ViewerSource.Timeline
)
