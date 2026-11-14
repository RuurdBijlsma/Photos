package dev.ruurd.photos.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

object PhotosRadii {
    val Sheet = 30.dp
    val Dialog = 28.dp
    val Card = 24.dp          // Daily cards, Album cards, Month outer corners
    val NestedCard = 16.dp    // Concentric inner content (24.dp - 8.dp padding)
    val TileOverlay = 6.dp    // Video duration badge inside grid cell
    val GridTile = 0.dp       // Sharp inner photo grid items
    val GridGap = 2.dp
}

fun concentricRadius(outer: Dp, padding: Dp, min: Dp = 8.dp): Dp =
    (outer - padding).coerceAtLeast(min)

val PhotosShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(30.dp)
)
