package dev.ruurd.photos.features.viewer.components

import android.content.Context
import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import coil3.BitmapImage
import coil3.memory.MemoryCache
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import coil3.size.Dimension
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaImageRequests
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import java.io.File
import coil3.size.Size as CoilSize

/**
 * The lightweight preview a page shows first: local thumbnail, the already-warmed original when
 * mounted post-rotation, or the 1440p cloud AVIF.
 */
@Composable
internal fun rememberPhotoPreviewRequest(
    item: TimelineMediaProjection,
    baseUrl: String,
    initialFullResFile: File?,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onGainmapChanged: (Boolean) -> Unit,
): ImageRequest? {
    val context = LocalContext.current
    return remember(item, baseUrl, initialFullResFile, memoryCacheKeyRegistry) {
        if (item.localContentUri == null && initialFullResFile != null && initialFullResFile.exists()) {
            val heightPx = MediaImageRequests.previewHeightPx(item.isVideo)
            ImageRequest.Builder(context)
                .data(Uri.fromFile(initialFullResFile))
                .size(CoilSize(Dimension.Undefined, Dimension.Pixels(heightPx)))
                .diskCachePolicy(CachePolicy.DISABLED)
                .apply {
                    memoryCacheKeyRegistry.get(item.id)?.let(::placeholderMemoryCacheKey)
                }
                .listener(
                    onSuccess = { _, result ->
                        result.memoryCacheKey?.let { memoryCacheKeyRegistry.put(item.id, it) }
                        onGainmapChanged((result.image as? BitmapImage)?.bitmap?.hasGainmap() == true)
                    },
                )
                .build()
        } else {
            MediaImageRequests.previewRequest(
                context = context,
                localContentUri = item.localContentUri,
                cloudId = item.cloudId,
                isVideo = item.isVideo,
                hasThumbnails = item.hasThumbnails,
                baseUrl = baseUrl,
                placeholderKey = memoryCacheKeyRegistry.get(item.id),
                onCacheKey = { memoryCacheKeyRegistry.put(item.id, it) },
                onImageSuccess = { image ->
                    onGainmapChanged((image as? BitmapImage)?.bitmap?.hasGainmap() == true)
                },
            )
        }
    }
}

/**
 * The full-res original on its own zoom layer, so Telephoto never swaps one `ZoomableState` from
 * a painter delegate to a subsampling delegate.
 */
@Composable
internal fun rememberPhotoFullResRequest(
    item: TimelineMediaProjection,
    data: Any?,
    fullResFile: File?,
    localUri: Uri?,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onLocalFullResFailed: () -> Unit,
): ImageRequest {
    val context = LocalContext.current
    return remember(item.id, data, fullResFile, localUri) {
        // Omit placeholderMemoryCacheKey on the dual-layer fullResZoomableState so it doesn't
        // emit a PainterDelegate before SubSamplingDelegate; the preview layer covers it.
        buildFullResImageRequest(
            context = context,
            itemId = item.id,
            data = data,
            fullResFile = fullResFile,
            localUri = localUri,
            placeholderKey = null,
            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
            onLocalFullResFailed = onLocalFullResFailed,
        )
    }
}

/** In-place upgrade for a page the user zoomed before the original finished loading. */
@Composable
internal fun rememberPhotoInPlaceUpgradeRequest(
    item: TimelineMediaProjection,
    upgradedInPreview: Boolean,
    data: Any?,
    fullResFile: File?,
    localUri: Uri?,
    previewRequest: ImageRequest?,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onLocalFullResFailed: () -> Unit,
): ImageRequest? {
    val context = LocalContext.current
    return remember(
        item.id,
        upgradedInPreview,
        data,
        fullResFile,
        localUri,
        previewRequest,
        memoryCacheKeyRegistry,
    ) {
        if (upgradedInPreview && data != null) {
            buildFullResImageRequest(
                context = context,
                itemId = item.id,
                data = data,
                fullResFile = fullResFile,
                localUri = localUri,
                placeholderKey = memoryCacheKeyRegistry.get(item.id),
                memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                onLocalFullResFailed = onLocalFullResFailed,
            )
        } else {
            previewRequest
        }
    }
}

private fun buildFullResImageRequest(
    context: Context,
    itemId: String,
    data: Any?,
    fullResFile: File?,
    localUri: Uri?,
    placeholderKey: MemoryCache.Key?,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onLocalFullResFailed: () -> Unit,
): ImageRequest = ImageRequest.Builder(context)
    .data(data)
    .apply {
        // Full-res originals live in full_res_cache/, never in Coil's 1 GB image cache.
        if (fullResFile != null) diskCachePolicy(CachePolicy.DISABLED)
        placeholderKey?.let(::placeholderMemoryCacheKey)
    }
    .listener(
        onSuccess = { _, result ->
            result.memoryCacheKey?.let { key ->
                memoryCacheKeyRegistry.put(itemId, key)
            }
        },
        onError = { _, _ ->
            if (fullResFile == null && localUri != null) onLocalFullResFailed()
        },
    )
    .build()