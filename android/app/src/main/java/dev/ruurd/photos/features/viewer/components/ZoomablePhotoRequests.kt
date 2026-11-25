package dev.ruurd.photos.features.viewer.components

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import coil3.BitmapImage
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaImageRequests
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import java.io.File

/** The lightweight preview a page shows first: local thumbnail, or the 1440p cloud AVIF. */
@Composable
internal fun rememberPhotoPreviewRequest(
    item: TimelineMediaProjection,
    baseUrl: String,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onGainmapChanged: (Boolean) -> Unit,
): ImageRequest? {
    val context = LocalContext.current
    return remember(item, baseUrl, memoryCacheKeyRegistry) {
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
): ImageRequest? {
    val context = LocalContext.current
    return remember(item.id, data, fullResFile, localUri) {
        ImageRequest.Builder(context)
            .data(data)
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
                onError = { _, _ ->
                    if (fullResFile == null && localUri != null) onLocalFullResFailed()
                },
            )
            .build()
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
            val placeholderKey = memoryCacheKeyRegistry.get(item.id)
            ImageRequest.Builder(context)
                .data(data)
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
                    onError = { _, _ ->
                        if (fullResFile == null && localUri != null) onLocalFullResFailed()
                    },
                )
                .build()
        } else {
            previewRequest
        }
    }
}
