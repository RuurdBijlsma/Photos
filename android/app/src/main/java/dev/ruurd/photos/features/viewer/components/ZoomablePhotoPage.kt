package dev.ruurd.photos.features.viewer.components

import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil3.request.ImageRequest
import coil3.request.crossfade
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.ui.theme.PhotosRadii
import me.saket.telephoto.zoomable.ZoomableState
import me.saket.telephoto.zoomable.coil3.ZoomableAsyncImage
import me.saket.telephoto.zoomable.rememberZoomableImageState
import me.saket.telephoto.zoomable.rememberZoomableState

@Composable
fun ZoomablePhotoPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    isActivePage: Boolean,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onTap: () -> Unit,
    modifier: Modifier = Modifier,
    onZoomFractionChanged: (Float) -> Unit = {}
) {
    val context = LocalContext.current
    val zoomableState = rememberZoomableState()

    // Reset zoom when swiping off-page
    LaunchedEffect(isActivePage) {
        if (!isActivePage) {
            zoomableState.resetZoom()
            onZoomFractionChanged(0f)
        }
    }

    val zoomFraction = zoomableState.zoomFraction ?: 0f
    val isZoomed = zoomFraction > 0.01f

    // Continuously observe zoom fraction on active page so HorizontalPager userScrollEnabled
    // is immediately disabled when zoomed in, preventing accidental paging while panning
    LaunchedEffect(zoomableState, isActivePage) {
        if (isActivePage) {
            snapshotFlow { zoomableState.zoomFraction ?: 0f }.collect { fraction ->
                onZoomFractionChanged(fraction)
            }
        }
    }

    // Active page upgrades to full-res immediately so opening a photo doesn't flash
    // a low-res thumbnail before swapping to high-res. Offscreen neighbor pages stay on lightweight preview.
    var isUpgradedToFullRes by remember(item.id) { mutableStateOf(isActivePage) }

    LaunchedEffect(item.id, isActivePage, isZoomed) {
        if (item.localContentUri != null && !item.isVideo && !isUpgradedToFullRes) {
            if (isActivePage || isZoomed) {
                isUpgradedToFullRes = true
            }
        }
    }

    val localUri = remember(item.localContentUri) { item.localContentUri?.let(Uri::parse) }
    val cloudFallbackUrl = remember(item, baseUrl) {
        when {
            !item.isVideo && !item.cloudId.isNullOrEmpty() -> {
                if (item.hasThumbnails) {
                    "$baseUrl/thumbnails/${item.cloudId}/1440p.avif"
                } else {
                    "$baseUrl/api/photos/${item.cloudId}/thumbnail?size=1440"
                }
            }
            !item.cloudId.isNullOrEmpty() -> {
                if (item.hasThumbnails) {
                    "$baseUrl/thumbnails/${item.cloudId}/720p.avif"
                } else {
                    "$baseUrl/api/photos/${item.cloudId}/thumbnail?size=720"
                }
            }
            else -> null
        }
    }

    val imageModel = remember(item, isUpgradedToFullRes, localUri, cloudFallbackUrl) {
        when {
            // Local item upgraded to full resolution
            localUri != null && isUpgradedToFullRes -> localUri
            // Local item preview (1440p thumbnail for photos, 720p for videos)
            localUri != null -> MediaStoreThumbnail(
                uri = localUri,
                cloudId = item.cloudId,
                cloudFallbackUrl = cloudFallbackUrl,
                isVideo = item.isVideo
            )
            // Cloud-only preview
            else -> cloudFallbackUrl
        }
    }

    val request = remember(imageModel, isUpgradedToFullRes) {
        ImageRequest.Builder(context)
            .data(imageModel)
            .apply {
                if (!isUpgradedToFullRes) {
                    val targetHeight = if (item.isVideo) 720 else 1440
                    size(targetHeight, targetHeight)
                }
                // Cheap already-decoded bitmap stays on screen until the next tier arrives, so
                // swapping preview -> full-res fades instead of blinking to black.
                memoryCacheKeyRegistry.get(item.id)?.let(::placeholderMemoryCacheKey)
            }
            .crossfade(CROSSFADE_MS)
            .listener(
                onSuccess = { _, result ->
                    result.memoryCacheKey?.let { key ->
                        memoryCacheKeyRegistry.put(item.id, key)
                    }
                }
            )
            .build()
    }

    val zoomableImageState = rememberZoomableImageState(zoomableState)

    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        ZoomableAsyncImage(
            model = request,
            contentDescription = null,
            state = zoomableImageState,
            onClick = { onTap() },
            contentScale = ContentScale.Fit,
            modifier = Modifier.fillMaxSize()
        )

        // Video overlay badge if video
        if (item.isVideo) {
            Row(
                modifier = Modifier
                    .align(Alignment.Center)
                    .background(
                        color = Color.Black.copy(alpha = 0.6f),
                        shape = RoundedCornerShape(PhotosRadii.Card)
                    )
                    .padding(horizontal = 16.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.PlayArrow,
                    contentDescription = "Video",
                    tint = Color.White,
                    modifier = Modifier.size(36.dp)
                )
                if (item.durationMs != null && item.durationMs > 0) {
                    Spacer(modifier = Modifier.width(6.dp))
                    val totalSec = item.durationMs / 1000
                    val min = totalSec / 60
                    val sec = totalSec % 60
                    Text(
                        text = "%d:%02d".format(min, sec),
                        style = MaterialTheme.typography.titleMedium,
                        color = Color.White
                    )
                }
            }
        }
    }
}

private const val CROSSFADE_MS = 180
