package dev.ruurd.photos.core.image

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInRoot

/**
 * Lightweight non-observable registry of currently placed timeline tile [LayoutCoordinates].
 * Stores only coordinate references during scroll (zero Rect/matrix allocations) and resolves
 * [boundsInRoot] on demand when opening or dismissing the media viewer.
 */
class TimelineTileBoundsRegistry {
    private val coordinates = HashMap<String, LayoutCoordinates>()

    fun register(id: String, layoutCoordinates: LayoutCoordinates) {
        coordinates[id] = layoutCoordinates
    }

    fun unregister(id: String, layoutCoordinates: LayoutCoordinates? = null) {
        if (layoutCoordinates == null || coordinates[id] === layoutCoordinates) {
            coordinates.remove(id)
        }
    }

    fun boundsInRoot(id: String): Rect? {
        val coords = coordinates[id] ?: return null
        if (!coords.isAttached) {
            coordinates.remove(id)
            return null
        }
        return coords.boundsInRoot()
    }

    fun clear() {
        coordinates.clear()
    }
}

/** Provided once by the app root; the viewer reads the tile bounds the timeline registered. */
val LocalTimelineTileBoundsRegistry = staticCompositionLocalOf<TimelineTileBoundsRegistry> {
    error("No TimelineTileBoundsRegistry provided")
}