package dev.ruurd.photos.core.image

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInRoot

@Immutable
data class TileCornerRadii(
    val topStartDp: Float = 0f,
    val topEndDp: Float = 0f,
    val bottomEndDp: Float = 0f,
    val bottomStartDp: Float = 0f,
) {
    val isZero: Boolean
        get() = topStartDp <= 0f && topEndDp <= 0f && bottomEndDp <= 0f && bottomStartDp <= 0f

    companion object {
        val Zero = TileCornerRadii()
    }
}

fun lerp(
    start: TileCornerRadii,
    stop: TileCornerRadii,
    fraction: Float,
): TileCornerRadii = TileCornerRadii(
    topStartDp = (start.topStartDp + (stop.topStartDp - start.topStartDp) * fraction).coerceAtLeast(0f),
    topEndDp = (start.topEndDp + (stop.topEndDp - start.topEndDp) * fraction).coerceAtLeast(0f),
    bottomEndDp = (start.bottomEndDp + (stop.bottomEndDp - start.bottomEndDp) * fraction).coerceAtLeast(0f),
    bottomStartDp = (start.bottomStartDp + (stop.bottomStartDp - start.bottomStartDp) * fraction).coerceAtLeast(0f),
)

/**
 * Lightweight non-observable registry of currently placed timeline tile [LayoutCoordinates] and
 * their outer month-block corner radii. Stores only coordinate references during scroll (zero
 * Rect/matrix allocations) and resolves [boundsInRoot] on demand when opening or dismissing the
 * media viewer.
 */
class TimelineTileBoundsRegistry {
    private val coordinates = HashMap<String, LayoutCoordinates>()
    private val corners = HashMap<String, TileCornerRadii>()

    fun register(
        id: String,
        layoutCoordinates: LayoutCoordinates,
        cornerRadii: TileCornerRadii = TileCornerRadii.Zero,
    ) {
        coordinates[id] = layoutCoordinates
        corners[id] = cornerRadii
    }

    fun unregister(id: String, layoutCoordinates: LayoutCoordinates? = null) {
        if (layoutCoordinates == null || coordinates[id] === layoutCoordinates) {
            coordinates.remove(id)
            corners.remove(id)
        }
    }

    fun boundsInRoot(id: String): Rect? {
        val coords = coordinates[id] ?: return null
        if (!coords.isAttached) {
            coordinates.remove(id)
            corners.remove(id)
            return null
        }
        return coords.boundsInRoot()
    }

    fun cornerRadii(id: String): TileCornerRadii = corners[id] ?: TileCornerRadii.Zero

    fun clear() {
        coordinates.clear()
        corners.clear()
    }
}

/** Provided once by the app root; the viewer reads the tile bounds the timeline registered. */
val LocalTimelineTileBoundsRegistry = staticCompositionLocalOf<TimelineTileBoundsRegistry> {
    error("No TimelineTileBoundsRegistry provided")
}