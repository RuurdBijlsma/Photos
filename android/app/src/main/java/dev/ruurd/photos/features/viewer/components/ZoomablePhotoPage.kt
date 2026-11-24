package dev.ruurd.photos.features.viewer.components

import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import coil3.SingletonImageLoader
import coil3.compose.asPainter
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.MediaImageRequests
import kotlinx.coroutines.delay
import me.saket.telephoto.zoomable.DoubleClickToZoomListener
import me.saket.telephoto.zoomable.ZoomSpec
import me.saket.telephoto.zoomable.coil3.ZoomableAsyncImage
import me.saket.telephoto.zoomable.rememberZoomableImageState
import me.saket.telephoto.zoomable.rememberZoomableState
import java.io.File

@Composable
fun ZoomablePhotoPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    isActivePage: Boolean,
    isSettledPage: Boolean,
    isOpeningTransition: Boolean,
    onTap: () -> Unit,
    modifier: Modifier = Modifier,
    /** Downloaded `/download` original for a cloud-only photo; `null` while it is not ready. */
    fullResFile: File? = null,
    onFullResRequested: () -> Unit = {},
    onZoomFractionChanged: (Float) -> Unit = {},
) {
    val context = LocalContext.current
    val imageLoader = SingletonImageLoader.get(context)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val zoomSpec = remember { ZoomSpec(maxZoomFactor = MAX_ZOOM_FACTOR) }
    val previewZoomableState = rememberZoomableState(zoomSpec = zoomSpec)
    val previewImageState = rememberZoomableImageState(previewZoomableState)

    val localUri = remember(item.localContentUri) { item.localContentUri?.let(Uri::parse) }
    // Local photos upgrade after settling; cloud-only photos upgrade once /download has landed.
    val canUpgrade = !item.isVideo && (localUri != null || fullResFile != null)
    var isUpgradedToFullRes by remember(item.id) { mutableStateOf(false) }
    var upgradedInPreview by remember(item.id) { mutableStateOf(false) }

    val previewZoomFraction = previewZoomableState.zoomFraction ?: 0f
    val isPreviewZoomed = previewZoomFraction > 0.01f

    // Use a separate ZoomableState for full-res at 1x so Telephoto never transitions one
    // ZoomableState from PainterDelegate to SubSamplingDelegate (avoiding a 2-frame shift and
    // low-res flash). If the user zooms early before full-res finishes, upgrade previewImageState
    // in-place so their zoom/pan isn't snapped back to 1x.
    val showDualOverlay = canUpgrade && isUpgradedToFullRes && !upgradedInPreview
    val fullResZoomableState = if (showDualOverlay) {
        rememberZoomableState(zoomSpec = zoomSpec)
    } else {
        null
    }
    val fullResImageState = fullResZoomableState?.let { rememberZoomableImageState(it) }

    // Latch once full quality is reached so zooming in later (which briefly swaps higher-detail
    // tiles) never tears down fullResImageState or swaps back to previewImageState mid-pinch.
    var hasDisplayedFullResInFullQuality by remember(item.id, showDualOverlay) {
        mutableStateOf(false)
    }
    val isFullResFullQuality = fullResImageState?.isImageDisplayedInFullQuality == true
    LaunchedEffect(item.id, showDualOverlay, isFullResFullQuality) {
        if (showDualOverlay && isFullResFullQuality) {
            hasDisplayedFullResInFullQuality = true
        }
    }

    val fullResReady = showDualOverlay &&
            hasDisplayedFullResInFullQuality &&
            !isPreviewZoomed

    val activeZoomableState = if (fullResReady && fullResZoomableState != null) {
        fullResZoomableState
    } else {
        previewZoomableState
    }

    // Reset zoom and revert offscreen pages back to lightweight previews.
    LaunchedEffect(isActivePage) {
        if (!isActivePage) {
            previewZoomableState.resetZoom()
            fullResZoomableState?.resetZoom()
            upgradedInPreview = false
            onZoomFractionChanged(0f)
        }
    }

    LaunchedEffect(activeZoomableState, isActivePage) {
        if (isActivePage) {
            snapshotFlow { activeZoomableState.zoomFraction ?: 0f }.collect { fraction ->
                onZoomFractionChanged(fraction)
            }
        }
    }

    // Two-tier loading: keep N ± 1 offscreen neighbors, opening transitions, and mid-swipe pages
    // on lightweight previews, upgrading to full-res after settling or immediately on zoom.
    LaunchedEffect(
        item.id,
        item.isVideo,
        canUpgrade,
        localUri,
        fullResFile,
        isSettledPage,
        isPreviewZoomed,
        isActivePage,
        isOpeningTransition,
    ) {
        if (!isActivePage) {
            isUpgradedToFullRes = false
            return@LaunchedEffect
        }
        if (isOpeningTransition) {
            return@LaunchedEffect
        }
        if (!item.isVideo && isPreviewZoomed) {
            if (localUri == null && fullResFile == null) {
                onFullResRequested()
            }
            if (canUpgrade) {
                isUpgradedToFullRes = true
                upgradedInPreview = true
            }
        } else if (canUpgrade && isSettledPage) {
            if (localUri == null) {
                isUpgradedToFullRes = true
            } else {
                delay(FULL_RES_SETTLE_DELAY_MS)
                isUpgradedToFullRes = true
            }
        } else if (!canUpgrade) {
            isUpgradedToFullRes = false
        }
    }

    // Synchronous O(1) memory-cache painter so frame 1 is never blank while ZoomableAsyncImage resolves.
    val syncFallbackPainter = remember(item.id, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
            ?.asPainter(context)
    }

    val previewRequest = remember(item, baseUrl, memoryCacheKeyRegistry) {
        MediaImageRequests.previewRequest(
            context = context,
            localContentUri = item.localContentUri,
            cloudId = item.cloudId,
            isVideo = item.isVideo,
            hasThumbnails = item.hasThumbnails,
            baseUrl = baseUrl,
            placeholderKey = memoryCacheKeyRegistry.get(item.id),
            onCacheKey = { memoryCacheKeyRegistry.put(item.id, it) },
        )
    }

    val fullResData = fullResFile?.let(Uri::fromFile) ?: localUri
    val fullResRequest = remember(item.id, fullResData) {
        ImageRequest.Builder(context)
            .data(fullResData)
            .apply {
                // Full-res originals live in full_res_cache/, never in Coil's 1 GB image cache.
                if (fullResFile != null) diskCachePolicy(CachePolicy.DISABLED)
            }
            // Omit placeholderMemoryCacheKey on the dual-layer fullResZoomableState so it doesn't
            // emit a PainterDelegate before SubSamplingDelegate; the preview layer covers it.
            .listener(
                onSuccess = { _, result ->
                    result.memoryCacheKey?.let { key ->
                        memoryCacheKeyRegistry.put(item.id, key)
                    }
                },
            )
            .build()
    }

    val inPlaceUpgradeRequest = if (upgradedInPreview && fullResData != null) {
        val placeholderKey = remember(item.id, memoryCacheKeyRegistry) {
            memoryCacheKeyRegistry.get(item.id)
        }
        remember(item.id, fullResData, placeholderKey) {
            ImageRequest.Builder(context)
                .data(fullResData)
                .apply {
                    if (fullResFile != null) diskCachePolicy(CachePolicy.DISABLED)
                    placeholderKey?.let(::placeholderMemoryCacheKey)
                }
                .listener(
                    onSuccess = { _, result ->
                        result.memoryCacheKey?.let { key ->
                            memoryCacheKeyRegistry.put(item.id, key)
                        }
                    },
                )
                .build()
        }
    } else {
        previewRequest
    }

    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        if (syncFallbackPainter != null && !previewImageState.isImageDisplayed && !fullResReady) {
            Image(
                painter = syncFallbackPainter,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }

        if (showDualOverlay && fullResImageState != null) {
            ZoomableAsyncImage(
                model = fullResRequest,
                contentDescription = null,
                state = fullResImageState,
                onClick = { onTap() },
                onDoubleClick = DoubleClickToZoomListener.cycle(DOUBLE_TAP_MAX_ZOOM_FACTOR),
                contentScale = ContentScale.Fit,
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { alpha = if (fullResReady) 1f else 0f },
            )
        }

        if (!fullResReady) {
            ZoomableAsyncImage(
                model = inPlaceUpgradeRequest,
                contentDescription = null,
                state = previewImageState,
                onClick = { onTap() },
                onDoubleClick = DoubleClickToZoomListener.cycle(DOUBLE_TAP_MAX_ZOOM_FACTOR),
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

private const val FULL_RES_SETTLE_DELAY_MS = 180L
private const val MAX_ZOOM_FACTOR = 8f
private const val DOUBLE_TAP_MAX_ZOOM_FACTOR = 2.5f