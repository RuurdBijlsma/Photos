package dev.ruurd.photos.features.viewer.components

import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import coil3.BitmapImage
import coil3.SingletonImageLoader
import coil3.compose.AsyncImage
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import kotlinx.coroutines.delay
import me.saket.telephoto.zoomable.DoubleClickToZoomListener
import me.saket.telephoto.zoomable.ZoomSpec
import me.saket.telephoto.zoomable.ZoomableContentLocation
import me.saket.telephoto.zoomable.ZoomableImageState
import me.saket.telephoto.zoomable.ZoomableState
import me.saket.telephoto.zoomable.coil3.ZoomableAsyncImage
import me.saket.telephoto.zoomable.rememberZoomableImageState
import me.saket.telephoto.zoomable.rememberZoomableState
import me.saket.telephoto.zoomable.zoomable
import java.io.File

@Composable
fun ZoomablePhotoPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    isActivePage: Boolean,
    isSettledPage: Boolean,
    isOffscreen: Boolean,
    isOpeningTransition: Boolean,
    onTap: () -> Unit,
    modifier: Modifier = Modifier,
    imageRevision: Int = 0,
    /** Downloaded `/download` original for a cloud-only photo; `null` while it is not ready. */
    fullResFile: File? = null,
    /** A programmatic info-panel open waiting on this page, so it can reset zoom first. */
    isInfoPanelRequested: Boolean = false,
    onFullResRequested: () -> Unit = {},
    onZoomFractionChanged: (Float) -> Unit = {},
    onHdrStateChanged: (hasPreviewGainmap: Boolean, hasSubSamplingLayer: Boolean) -> Unit = { _, _ -> },
    onSyncWindowColorMode: () -> Unit = {},
    onPageDisposed: () -> Unit = {},
) {
    val context = LocalContext.current
    val imageLoader = SingletonImageLoader.get(context)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current
    val zoomSpec = remember { ZoomSpec(maxZoomFactor = MAX_ZOOM_FACTOR) }
    val previewZoomableState = rememberZoomableState(zoomSpec = zoomSpec)
    val previewImageState = rememberZoomableImageState(previewZoomableState)

    val localUri = remember(item.localContentUri) { item.localContentUri?.let(Uri::parse) }
    var localFullResFailed by remember(item.id, localUri, imageRevision) { mutableStateOf(false) }
    val effectiveLocalUri = localUri?.takeUnless { localFullResFailed }

    // Local photos upgrade after settling; cloud-only (or unreadable synced) photos upgrade once /download has landed.
    val canUpgrade = !item.isVideo && (effectiveLocalUri != null || fullResFile != null)
    var isUpgradedToFullRes by remember(item.id, imageRevision) { mutableStateOf(false) }
    var upgradedInPreview by remember(item.id, imageRevision) { mutableStateOf(false) }

    var previewIntrinsicSize by remember(item.id, imageRevision) { mutableStateOf(Size.Unspecified) }
    var isPreviewDisplayed by remember(item.id, imageRevision) { mutableStateOf(false) }

    var previewHasGainmap by remember(item.id, imageRevision, memoryCacheKeyRegistry, imageLoader) {
        val cachedHasGainmap = memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image as? BitmapImage }
            ?.bitmap
            ?.hasGainmap()
            ?: false
        mutableStateOf(cachedHasGainmap)
    }

    val previewZoomFraction = previewZoomableState.zoomFraction ?: 0f
    val isPreviewZoomed = previewZoomFraction > 0.01f

    // If a synced item's local URI failed (e.g. partial access revocation or deleted file), fall back
    // to the preview/cloud pipeline and trigger a /download upgrade instead of blanking out on zoom.
    LaunchedEffect(localFullResFailed, item.cloudId, fullResFile) {
        if (localFullResFailed) {
            upgradedInPreview = false
            isUpgradedToFullRes = false
            if (item.cloudId != null && fullResFile == null) {
                onFullResRequested()
            }
        }
    }

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
    var hasDisplayedFullResInFullQuality by remember(item.id, imageRevision, showDualOverlay) {
        mutableStateOf(false)
    }
    val isFullResFullQuality = fullResImageState?.isImageDisplayedInFullQuality == true
    LaunchedEffect(item.id, imageRevision, showDualOverlay, isFullResFullQuality) {
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

    // Reset zoom and revert offscreen pages back to lightweight previews once fully off screen.
    LaunchedEffect(isOffscreen) {
        if (isOffscreen) {
            previewZoomableState.resetZoom()
            fullResZoomableState?.resetZoom()
            upgradedInPreview = false
            onZoomFractionChanged(0f)
        }
    }

    // Programmatic info open (e.g. "About" while zoomed in) resets zoom so the panel can slide up.
    LaunchedEffect(isActivePage, isInfoPanelRequested) {
        if (isActivePage && isInfoPanelRequested) {
            previewZoomableState.resetZoom()
            fullResZoomableState?.resetZoom()
        }
    }

    LaunchedEffect(activeZoomableState, isActivePage) {
        if (isActivePage) {
            snapshotFlow { activeZoomableState.zoomFraction ?: 0f }.collect { fraction ->
                onZoomFractionChanged(fraction)
            }
        }
    }

    // Two-tier loading: keep N ± 1 offscreen neighbors, opening transitions, and incoming pages
    // on lightweight previews, upgrading to full-res after settling or immediately on zoom. Once
    // upgraded, keep the full-res layer mounted until the page leaves the viewport so swiping
    // away never drops HDR/gainmap mid-transition.
    LaunchedEffect(
        item.id,
        imageRevision,
        item.isVideo,
        canUpgrade,
        effectiveLocalUri,
        fullResFile,
        isSettledPage,
        isPreviewZoomed,
        isActivePage,
        isOffscreen,
        isOpeningTransition,
    ) {
        if (isOffscreen) {
            isUpgradedToFullRes = false
            return@LaunchedEffect
        }
        if (isOpeningTransition) {
            return@LaunchedEffect
        }
        if (isActivePage && !item.isVideo && isPreviewZoomed) {
            if (effectiveLocalUri == null && fullResFile == null) {
                onFullResRequested()
            }
            if (canUpgrade) {
                isUpgradedToFullRes = true
                upgradedInPreview = true
            }
        } else if (isActivePage && canUpgrade && isSettledPage) {
            if (effectiveLocalUri == null) {
                isUpgradedToFullRes = true
            } else {
                delay(FULL_RES_SETTLE_DELAY_MS)
                isUpgradedToFullRes = true
            }
        } else if (!canUpgrade) {
            isUpgradedToFullRes = false
        }
    }

    // Synchronous O(1) memory-cache painter so frame 1 is never blank while Coil resolves.
    val syncFallbackPainter = remember(item.id, imageRevision, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.peekPainter(imageLoader, context, item.id)
    }

    // Snapshot fullResFile at mount time so post-rotation cloud_new uses its warmed local file
    // immediately for the preview layer instead of hitting /thumbnail, while normal swipes don't
    // restart previewRequest when fullResFile arrives asynchronously.
    val initialFullResFile = remember(item.id, imageRevision) { fullResFile }
    val previewRequest = rememberPhotoPreviewRequest(
        item = item,
        baseUrl = baseUrl,
        initialFullResFile = initialFullResFile,
        memoryCacheKeyRegistry = memoryCacheKeyRegistry,
    ) { hasGainmap ->
        previewHasGainmap = hasGainmap
    }

    val fullResData = fullResFile?.let(Uri::fromFile) ?: effectiveLocalUri
    val fullResRequest = rememberPhotoFullResRequest(
        item = item,
        data = fullResData,
        fullResFile = fullResFile,
        localUri = localUri,
        memoryCacheKeyRegistry = memoryCacheKeyRegistry,
    ) {
        localFullResFailed = true
    }

    val inPlaceUpgradeRequest = rememberPhotoInPlaceUpgradeRequest(
        item = item,
        upgradedInPreview = upgradedInPreview,
        data = fullResData,
        fullResFile = fullResFile,
        localUri = localUri,
        previewRequest = previewRequest,
        memoryCacheKeyRegistry = memoryCacheKeyRegistry,
    ) {
        localFullResFailed = true
    }

    // Cloud-only 1440p.avif previews stay on AsyncImage + Modifier.zoomable so Telephoto does not
    // open a Coil DiskCache snapshot for an un-subsamplable AVIF (saket/telephoto#181).
    val useSimpleZoomablePreview = effectiveLocalUri == null && !upgradedInPreview
    LaunchedEffect(useSimpleZoomablePreview, previewZoomableState, previewIntrinsicSize) {
        if (useSimpleZoomablePreview && previewIntrinsicSize.isSpecified) {
            previewZoomableState.setContentLocation(
                ZoomableContentLocation.scaledInsideAndCenterAligned(previewIntrinsicSize),
            )
        }
    }

    val hasSubSamplingLayer = (showDualOverlay && fullResImageState?.isImageDisplayed == true) ||
        (upgradedInPreview && fullResData != null)
    val currentOnHdrStateChanged by rememberUpdatedState(onHdrStateChanged)
    val currentOnSyncWindowColorMode by rememberUpdatedState(onSyncWindowColorMode)
    val currentOnPageDisposed by rememberUpdatedState(onPageDisposed)

    // SideEffect runs after all DisposableEffect.onDispose blocks in the same frame, so an
    // outgoing page disposing its SubSamplingImage cannot leave window.colorMode in SDR when
    // the incoming visible page already carries an HDR gainmap preview.
    SideEffect {
        currentOnHdrStateChanged(previewHasGainmap, hasSubSamplingLayer)
        currentOnSyncWindowColorMode()
    }

    DisposableEffect(item.id, imageRevision) {
        onDispose {
            currentOnPageDisposed()
            currentOnSyncWindowColorMode()
        }
    }

    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        val previewReady = isPreviewDisplayed || previewImageState.isImageDisplayed
        if (syncFallbackPainter != null && !previewReady && !fullResReady) {
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
            PreviewZoomableLayer(
                useSimplePreview = useSimpleZoomablePreview,
                previewRequest = previewRequest,
                inPlaceUpgradeRequest = inPlaceUpgradeRequest,
                previewZoomableState = previewZoomableState,
                previewImageState = previewImageState,
                onTap = onTap,
                onSimplePreviewReady = { intrinsicSize ->
                    isPreviewDisplayed = true
                    previewIntrinsicSize = intrinsicSize
                },
            )
        }
    }
}

@Composable
private fun PreviewZoomableLayer(
    useSimplePreview: Boolean,
    previewRequest: ImageRequest?,
    inPlaceUpgradeRequest: ImageRequest?,
    previewZoomableState: ZoomableState,
    previewImageState: ZoomableImageState,
    onTap: () -> Unit,
    onSimplePreviewReady: (Size) -> Unit,
) {
    if (useSimplePreview) {
        AsyncImage(
            model = previewRequest,
            contentDescription = null,
            contentScale = ContentScale.Fit,
            onSuccess = { state -> onSimplePreviewReady(state.painter.intrinsicSize) },
            modifier = Modifier
                .fillMaxSize()
                .zoomable(
                    state = previewZoomableState,
                    onClick = { onTap() },
                    onDoubleClick = DoubleClickToZoomListener.cycle(DOUBLE_TAP_MAX_ZOOM_FACTOR),
                ),
        )
    } else {
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

private const val FULL_RES_SETTLE_DELAY_MS = 180L
private const val MAX_ZOOM_FACTOR = 8f
private const val DOUBLE_TAP_MAX_ZOOM_FACTOR = 2.5f
