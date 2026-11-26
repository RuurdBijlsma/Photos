package dev.ruurd.photos.features.viewer.components

import androidx.annotation.OptIn
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.media3.common.util.UnstableApi
import androidx.media3.ui.compose.PlayerSurface
import androidx.media3.ui.compose.SURFACE_TYPE_TEXTURE_VIEW
import androidx.media3.ui.compose.modifiers.resizeWithContentScale
import androidx.media3.ui.compose.state.rememberPresentationState
import coil3.SingletonImageLoader
import coil3.compose.AsyncImage
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.LocalMemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.MediaImageRequests
import dev.ruurd.photos.domain.model.ResolvedVideoSource
import dev.ruurd.photos.domain.model.VideoUnavailableReason
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlinx.coroutines.delay

/**
 * Video page inside the viewer's `HorizontalPager`. Offscreen/unsettled pages render only the
 * lightweight thumbnail shutter; once settled, the single shared `ExoPlayer` attaches via
 * `PlayerSurface(SURFACE_TYPE_TEXTURE_VIEW)` pre-sized to `item.ratio` and hidden until both the
 * first frame and presentation `videoSizeDp` are ready. Tapping toggles chrome rather than
 * play/pause.
 */
@OptIn(UnstableApi::class)
@Composable
fun VideoPlayerPage(
    item: TimelineMediaProjection,
    baseUrl: String,
    isCurrentPage: Boolean,
    isSettledPage: Boolean,
    isClosing: Boolean,
    isInfoPanelActive: Boolean,
    isDragging: Boolean,
    videoController: ViewerVideoController,
    resolvedSource: ResolvedVideoSource,
    errorReason: VideoUnavailableReason?,
    onTap: () -> Unit,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val activeError = errorReason ?: (resolvedSource as? ResolvedVideoSource.Unavailable)?.reason
    val canAttachPlayer = isSettledPage && isCurrentPage && !isClosing && activeError == null

    LaunchedEffect(
        item.id,
        resolvedSource,
        canAttachPlayer,
        item.hasThumbnails,
        item.durationMs,
    ) {
        if (canAttachPlayer) {
            val shouldAutoPlay = !isInfoPanelActive && !isDragging && !isClosing
            videoController.loadOrUpdateSource(
                mediaId = item.id,
                source = resolvedSource,
                hasThumbnails = item.hasThumbnails,
                fallbackDurationMs = (item.durationMs ?: 0).toLong(),
                autoPlay = shouldAutoPlay,
            )
        }
    }

    var showBufferingSpinner by remember(item.id) { mutableStateOf(false) }
    val isBufferingActive = canAttachPlayer &&
        videoController.activeMediaId == item.id &&
        videoController.isBuffering
    LaunchedEffect(isBufferingActive, item.id) {
        if (isBufferingActive) {
            delay(BUFFERING_SPINNER_DELAY_MS)
            showBufferingSpinner = true
        } else {
            showBufferingSpinner = false
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null,
                onClick = onTap,
            ),
        contentAlignment = Alignment.Center,
    ) {
        if (canAttachPlayer) {
            // Guard against Video A's last frame or an un-resized TextureView flashing before
            // Video B's first frame and videoSizeDp are ready.
            val awaitingFirstFrame = videoController.activeMediaId != item.id ||
                (!videoController.hasRenderedFirstFrame && !videoController.isQualitySwitch)
            val presentationState = rememberPresentationState(
                player = videoController.player,
                keepContentOnReset = videoController.isQualitySwitch,
            )
            val showShutter = awaitingFirstFrame ||
                presentationState.coverSurface ||
                presentationState.videoSizeDp == null
            val fallbackSizeDp = remember(item.ratio) {
                item.ratio
                    .takeIf { it.isFinite() && it > 0f }
                    ?.let { ratio -> Size(ratio * 1000f, 1000f) }
            }
            val sourceSizeDp = presentationState.videoSizeDp ?: fallbackSizeDp

            PlayerSurface(
                player = videoController.player,
                surfaceType = SURFACE_TYPE_TEXTURE_VIEW,
                modifier = Modifier
                    .fillMaxSize()
                    .resizeWithContentScale(ContentScale.Fit, sourceSizeDp)
                    .graphicsLayer { alpha = if (showShutter) 0f else 1f },
            )

            if (showShutter) {
                VideoThumbnailShutter(
                    item = item,
                    baseUrl = baseUrl,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        } else {
            VideoThumbnailShutter(
                item = item,
                baseUrl = baseUrl,
                modifier = Modifier.fillMaxSize(),
            )
        }

        if (showBufferingSpinner) {
            CircularProgressIndicator(
                color = Color.White,
                strokeWidth = 3.dp,
                modifier = Modifier.size(44.dp),
            )
        }

        if (isSettledPage && activeError != null) {
            VideoErrorOverlay(
                reason = activeError,
                onRetry = onRetry,
                modifier = Modifier.padding(horizontal = 32.dp),
            )
        }
    }
}

@Composable
private fun VideoThumbnailShutter(
    item: TimelineMediaProjection,
    baseUrl: String,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val imageLoader = SingletonImageLoader.get(context)
    val memoryCacheKeyRegistry = LocalMemoryCacheKeyRegistry.current

    val syncFallbackPainter = remember(item.id, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.peekPainter(imageLoader, context, item.id)
    }
    var isPreviewReady by remember(item.id) { mutableStateOf(false) }

    val previewRequest = remember(item, baseUrl, memoryCacheKeyRegistry) {
        MediaImageRequests.previewRequest(
            context = context,
            localContentUri = item.localContentUri,
            cloudId = item.cloudId,
            isVideo = true,
            hasThumbnails = item.hasThumbnails,
            baseUrl = baseUrl,
            placeholderKey = memoryCacheKeyRegistry.get(item.id),
            onCacheKey = { key -> memoryCacheKeyRegistry.put(item.id, key) },
        )
    }

    Box(
        modifier = modifier,
        contentAlignment = Alignment.Center,
    ) {
        if (syncFallbackPainter != null && !isPreviewReady) {
            Image(
                painter = syncFallbackPainter,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }
        if (previewRequest != null) {
            AsyncImage(
                model = previewRequest,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                onSuccess = { isPreviewReady = true },
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

@Composable
private fun VideoErrorOverlay(
    reason: VideoUnavailableReason,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val message = when (reason) {
        VideoUnavailableReason.PROCESSING ->
            "Video is still processing on the server. Check back later."

        VideoUnavailableReason.MISSING_SOURCE ->
            "Video file is missing from server storage."

        VideoUnavailableReason.OFFLINE ->
            "You're offline. Connect to stream this video."

        VideoUnavailableReason.PLAYBACK_FAILED ->
            "Unable to play this video."
    }

    Surface(
        shape = RoundedCornerShape(PhotosRadii.Card),
        color = Color.Black.copy(alpha = 0.72f),
        modifier = modifier,
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = Color.White,
                textAlign = TextAlign.Center,
            )
            if (reason != VideoUnavailableReason.MISSING_SOURCE) {
                Spacer(modifier = Modifier.height(8.dp))
                TextButton(
                    onClick = onRetry,
                    colors = ButtonDefaults.textButtonColors(contentColor = Color.White),
                ) {
                    Text("Retry")
                }
            }
        }
    }
}

private const val BUFFERING_SPINNER_DELAY_MS = 250L
