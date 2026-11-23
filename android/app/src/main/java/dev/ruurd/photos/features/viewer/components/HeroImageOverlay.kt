package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import coil3.SingletonImageLoader
import coil3.compose.AsyncImage
import coil3.compose.asPainter
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.MediaImageRequests
import dev.ruurd.photos.core.image.TileCornerRadii
import kotlin.math.roundToInt

/**
 * The tile↔viewer hero image: a single bitmap placed at [rect] and cropped to it, so the open and
 * close animations can interpolate geometry and corner radii without the image re-laying out.
 */
@Composable
fun HeroImageOverlay(
    item: TimelineMediaProjection,
    baseUrl: String,
    rect: Rect,
    alpha: Float,
    modifier: Modifier = Modifier,
    cornerRadii: TileCornerRadii = TileCornerRadii.Zero,
    onImageReady: () -> Unit = {},
) {
    val context = LocalContext.current
    val imageLoader = SingletonImageLoader.get(context)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current

    val cachedPainter = remember(item.id, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
            ?.asPainter(context)
    }

    LaunchedEffect(item.id, cachedPainter) {
        if (cachedPainter != null) {
            onImageReady()
        }
    }

    val previewRequest = remember(item, baseUrl) {
        MediaImageRequests.previewRequest(
            context = context,
            localContentUri = item.localContentUri,
            cloudId = item.cloudId,
            isVideo = item.isVideo,
            hasThumbnails = item.hasThumbnails,
            baseUrl = baseUrl,
            onCacheKey = { key -> memoryCacheKeyRegistry.put(item.id, key) },
        )
    }

    val shape = if (cornerRadii.isZero) {
        RectangleShape
    } else {
        RoundedCornerShape(
            topStart = cornerRadii.topStartDp.dp,
            topEnd = cornerRadii.topEndDp.dp,
            bottomEnd = cornerRadii.bottomEndDp.dp,
            bottomStart = cornerRadii.bottomStartDp.dp,
        )
    }

    Box(
        modifier = modifier
            .offset { IntOffset(rect.left.roundToInt(), rect.top.roundToInt()) }
            .layout { measurable, _ ->
                val w = rect.width.roundToInt().coerceAtLeast(1)
                val h = rect.height.roundToInt().coerceAtLeast(1)
                val placeable = measurable.measure(Constraints.fixed(w, h))
                layout(w, h) {
                    placeable.place(0, 0)
                }
            }
            .graphicsLayer {
                this.alpha = alpha
                this.shape = shape
                this.clip = true
            },
    ) {
        if (cachedPainter != null) {
            Image(
                painter = cachedPainter,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        } else if (previewRequest != null) {
            AsyncImage(
                model = previewRequest,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                onSuccess = { onImageReady() },
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}