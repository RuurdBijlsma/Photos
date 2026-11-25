package dev.ruurd.photos.features.timeline.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.LocalTimelineTileBoundsRegistry
import dev.ruurd.photos.core.image.MediaImageRequests
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.domain.model.TimelineGridItemUiModel
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlin.math.roundToInt

/**
 * One justified-grid photo: the thumbnail request, the bounds/cache-key registry writes the viewer
 * needs, and the video/not-backed-up overlays. Size comes from the precomputed layout.
 */
@Composable
internal fun TimelineTile(
    item: TimelineGridItemUiModel,
    heightDp: Float,
    thumbnailTargetHeightPx: Int,
    baseUrl: String,
    isScrubbing: Boolean,
    isHidden: Boolean,
    cornerRadii: TileCornerRadii,
    onClick: (sourceRect: Rect?) -> Unit,
) {
    val context = LocalContext.current
    val density = LocalDensity.current.density
    val boundsRegistry = LocalTimelineTileBoundsRegistry.current
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val widthPx = (item.widthDp * density).roundToInt().coerceAtLeast(1)
    val heightPx = (heightDp * density).roundToInt().coerceAtLeast(1)

    // Plain holders, not state: updating these must not recompose the tile during scroll.
    val loadState = remember(item, thumbnailTargetHeightPx, baseUrl, widthPx, heightPx) {
        TileLoadState()
    }
    val coordsHolder = remember { arrayOfNulls<LayoutCoordinates>(1) }

    SideEffect {
        val coords = coordsHolder[0]
        if (coords != null && coords.isAttached) {
            boundsRegistry.register(item.id, coords, cornerRadii)
        }
    }

    DisposableEffect(item.id, boundsRegistry) {
        onDispose {
            boundsRegistry.unregister(item.id, coordsHolder[0])
            coordsHolder[0] = null
        }
    }

    var gateLoads by remember(item, thumbnailTargetHeightPx, baseUrl, widthPx, heightPx) {
        mutableStateOf(isScrubbing)
    }
    if (gateLoads) {
        LaunchedEffect(isScrubbing) {
            if (!isScrubbing && !loadState.succeeded) {
                gateLoads = false
            }
        }
    }

    val request = remember(
        item,
        thumbnailTargetHeightPx,
        baseUrl,
        widthPx,
        heightPx,
        gateLoads,
    ) {
        val model = MediaImageRequests.previewModel(
            localContentUri = item.localContentUri,
            cloudId = item.cloudId,
            isVideo = item.isVideo,
            hasThumbnails = item.hasThumbnails,
            baseUrl = baseUrl,
            heightPx = thumbnailTargetHeightPx,
            bypassPlatformThumbnail = false,
        )
        ImageRequest.Builder(context)
            .data(model)
            .size(widthPx, heightPx)
            .apply {
                if (gateLoads) {
                    // Fast scrubbing: reuse memory/disk caches without flooding MediaStore IPC or network.
                    memoryCachePolicy(CachePolicy.ENABLED)
                    diskCachePolicy(CachePolicy.READ_ONLY)
                    networkCachePolicy(CachePolicy.DISABLED)
                }
            }
            .build()
    }

    Box(
        modifier = Modifier
            .width(item.widthDp.dp)
            .height(heightDp.dp)
            .background(MaterialTheme.colorScheme.surfaceContainerHigh)
            .onPlaced { coords ->
                coordsHolder[0] = coords
                boundsRegistry.register(item.id, coords, cornerRadii)
            }
            .clickable {
                val rect = coordsHolder[0]?.takeIf { it.isAttached }?.boundsInRoot()
                onClick(rect)
            },
    ) {
        // Coil 3's default model equality ignores CachePolicy changes, so uncached tiles that were
        // skipped during scrubbing must re-mount once scrubbing stops.
        key(gateLoads) {
            AsyncImage(
                model = request,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                onSuccess = { result ->
                    loadState.succeeded = true
                    result.result.memoryCacheKey?.let { key ->
                        memoryCacheKeyRegistry.put(item.id, key)
                    }
                },
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { alpha = if (isHidden) 0f else 1f },
            )
        }

        if (item.isVideo && !isHidden) {
            Row(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(4.dp)
                    .background(
                        color = Color.Black.copy(alpha = 0.6f),
                        shape = RoundedCornerShape(PhotosRadii.TileOverlay),
                    )
                    .padding(horizontal = 6.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = Icons.Default.PlayArrow,
                    contentDescription = null,
                    tint = Color.White,
                    modifier = Modifier.size(12.dp),
                )
                if (item.durationLabel != null) {
                    Spacer(modifier = Modifier.width(2.dp))
                    Text(
                        text = item.durationLabel,
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White,
                    )
                }
            }
        }

        if (item.syncStatus == SyncStatus.LOCAL_ONLY && !isHidden) {
            Icon(
                imageVector = Icons.Default.CloudOff,
                contentDescription = "Not backed up",
                tint = Color.White,
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(4.dp)
                    .background(Color.Black.copy(alpha = 0.4f), CircleShape)
                    .padding(3.dp)
                    .size(11.dp),
            )
        }
    }
}

/** Mutable-without-recomposition flag for "this tile already rendered its image". */
private class TileLoadState {
    var succeeded = false
}
