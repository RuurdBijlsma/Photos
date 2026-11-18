package dev.ruurd.photos.features.viewer.components

import android.net.Uri
import androidx.compose.foundation.Image
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import coil3.compose.asPainter
import coil3.request.ImageRequest
import coil3.size.Dimension
import coil3.size.Size as CoilSize
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlinx.coroutines.delay
import me.saket.telephoto.zoomable.coil3.ZoomableAsyncImage
import me.saket.telephoto.zoomable.rememberZoomableImageState
import me.saket.telephoto.zoomable.rememberZoomableState

@Composable
fun ZoomablePhotoPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    isActivePage: Boolean,
    isSettledPage: Boolean,
    isOpeningTransition: Boolean,
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

    LaunchedEffect(zoomableState, isActivePage) {
        if (isActivePage) {
            snapshotFlow { zoomableState.zoomFraction ?: 0f }.collect { fraction ->
                onZoomFractionChanged(fraction)
            }
        }
    }

    // Two-tier loading: keep N ± 1 offscreen neighbors, opening transitions, and mid-swipe pages
    // on lightweight previews, upgrading local/synced photos to raw full-res only after settling or on zoom.
    val canUpgradeLocal = item.localContentUri != null && !item.isVideo
    var isUpgradedToFullRes by remember(item.id) { mutableStateOf(false) }

    LaunchedEffect(
        item.id,
        canUpgradeLocal,
        isSettledPage,
        isZoomed,
        isActivePage,
        isOpeningTransition
    ) {
        if (!canUpgradeLocal) {
            isUpgradedToFullRes = false
            return@LaunchedEffect
        }
        if (isOpeningTransition) {
            return@LaunchedEffect
        }
        if (isZoomed && isActivePage) {
            isUpgradedToFullRes = true
        } else if (isSettledPage) {
            delay(FULL_RES_SETTLE_DELAY_MS)
            isUpgradedToFullRes = true
        } else {
            isUpgradedToFullRes = false
        }
    }

    // Synchronous O(1) memory-cache painter so frame 1 is never blank while ZoomableAsyncImage resolves.
    val syncFallbackPainter = remember(item.id, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
            ?.asPainter(context)
    }

    val localUri = remember(item.localContentUri) { item.localContentUri?.let(Uri::parse) }
    val cloudFallbackUrl = remember(item, baseUrl) {
        val cloudId = item.cloudId?.takeIf { it.isNotBlank() }
        if (cloudId == null || baseUrl.isBlank()) {
            null
        } else {
            val targetHeight = if (item.isVideo) 720 else 1440
            if (item.hasThumbnails) {
                "$baseUrl/thumbnails/$cloudId/${targetHeight}p.avif"
            } else {
                "$baseUrl/api/photos/$cloudId/thumbnail?size=$targetHeight"
            }
        }
    }

    val imageModel = remember(item.id, isUpgradedToFullRes, localUri, cloudFallbackUrl) {
        when {
            localUri != null && isUpgradedToFullRes -> localUri
            localUri != null -> MediaStoreThumbnail(
                uri = localUri,
                cloudId = item.cloudId,
                cloudFallbackUrl = cloudFallbackUrl,
                isVideo = item.isVideo
            )
            else -> cloudFallbackUrl
        }
    }

    val isLocalPreview = localUri != null && !isUpgradedToFullRes
    val isCloudOnly = localUri == null
    val request = remember(item.id, imageModel, isLocalPreview, isCloudOnly, item.isVideo) {
        ImageRequest.Builder(context)
            .data(imageModel)
            .apply {
                val targetHeight = if (item.isVideo) 720 else 1440
                when {
                    isLocalPreview -> size(targetHeight, targetHeight)
                    // Preserve full 1440p/720p height on landscape cloud AVIFs instead of bounding width
                    isCloudOnly -> size(CoilSize(Dimension.Undefined, Dimension.Pixels(targetHeight)))
                }
                memoryCacheKeyRegistry.get(item.id)?.let(::placeholderMemoryCacheKey)
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
        if (syncFallbackPainter != null && !zoomableImageState.isImageDisplayed) {
            Image(
                painter = syncFallbackPainter,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize()
            )
        }

        ZoomableAsyncImage(
            model = request,
            contentDescription = null,
            state = zoomableImageState,
            onClick = { onTap() },
            contentScale = ContentScale.Fit,
            modifier = Modifier.fillMaxSize()
        )

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

private const val FULL_RES_SETTLE_DELAY_MS = 180L