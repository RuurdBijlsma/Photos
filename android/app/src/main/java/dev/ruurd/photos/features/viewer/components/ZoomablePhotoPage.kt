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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlinx.coroutines.delay
import me.saket.telephoto.zoomable.ZoomableState
import me.saket.telephoto.zoomable.coil3.ZoomableAsyncImage
import me.saket.telephoto.zoomable.rememberZoomableImageState
import me.saket.telephoto.zoomable.rememberZoomableState

@Composable
fun ZoomablePhotoPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    isActivePage: Boolean,
    isSettledPage: Boolean,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onTap: () -> Unit,
    modifier: Modifier = Modifier,
    onZoomableStateReady: (ZoomableState) -> Unit = {}
) {
    val context = LocalContext.current
    val zoomableState = rememberZoomableState()

    LaunchedEffect(zoomableState) {
        onZoomableStateReady(zoomableState)
    }

    // Reset zoom when swiping off-page
    LaunchedEffect(isActivePage) {
        if (!isActivePage) {
            zoomableState.resetZoom()
        }
    }

    val zoomFraction = zoomableState.zoomFraction ?: 0f
    val isZoomed = zoomFraction > 0.01f

    // Two-tier preview-first loading for local items:
    // Render lightweight MediaStoreThumbnail on frame 1 and N ± 1 offscreen pages.
    // Upgrade to full-res raw content:// Uri only ~150-250ms after settled or immediately when zoom begins.
    var isUpgradedToFullRes by remember(item.id) { mutableStateOf(false) }

    LaunchedEffect(item.id, isSettledPage, isZoomed) {
        if (item.localContentUri != null && !item.isVideo && !isUpgradedToFullRes) {
            if (isZoomed) {
                isUpgradedToFullRes = true
            } else if (isSettledPage) {
                delay(200L)
                isUpgradedToFullRes = true
            }
        }
    }

    val cachedKey = remember(item.id) { memoryCacheKeyRegistry.get(item.id) }

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

    val request = remember(imageModel, cachedKey, isUpgradedToFullRes) {
        ImageRequest.Builder(context)
            .data(imageModel)
            .apply {
                if (!isUpgradedToFullRes) {
                    val targetHeight = if (item.isVideo) 720 else 1440
                    size(targetHeight, targetHeight)
                }
                if (cachedKey != null) {
                    placeholderMemoryCacheKey(cachedKey)
                }
            }
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
