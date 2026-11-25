package dev.ruurd.photos.features.viewer.components

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.IntSize
import coil3.ImageLoader
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import kotlin.math.min

/**
 * Where the photo and the info panel sit inside the viewer: the fitted photo rect, the panel's rest
 * position, and how far each travels while the panel opens. Plain math over the container size, so
 * it is independent of gestures and animations.
 */
@Stable
internal class ViewerGeometry(
    private val imageLoader: ImageLoader,
    private val memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    private val sheetCornerOverlapPx: Float,
) {
    /** Assigned by the viewer on every composition and layout change. */
    var viewerSize: IntSize by mutableStateOf(IntSize.Zero)
    var statusBarHeightPx: Float by mutableFloatStateOf(0f)

    private val infoContentHeights = HashMap<String, Int>()

    fun infoContentHeight(mediaId: String): Int = infoContentHeights[mediaId] ?: 0

    fun setInfoContentHeight(mediaId: String, heightPx: Int) {
        infoContentHeights[mediaId] = heightPx
    }

    val sheetRestTopPx: Float
        get() {
            val containerH = viewerSize.height.toFloat()
            if (containerH <= 0f) return 0f
            return (containerH * SHEET_REST_TOP_FRACTION).coerceAtLeast(statusBarHeightPx)
        }

    fun fittedRect(item: TimelineMediaProjection): Rect = computeFittedRect(
        containerWidth = viewerSize.width.toFloat(),
        containerHeight = viewerSize.height.toFloat(),
        ratio = resolvePhotoRatio(item),
    )

    fun baseTopPx(item: TimelineMediaProjection): Float {
        val containerH = viewerSize.height.toFloat()
        if (containerH <= 0f || viewerSize.width <= 0) return 0f
        return fittedRect(item).bottom.coerceIn(0f, containerH)
    }

    fun sheetOverlapPx(item: TimelineMediaProjection): Float = min(
        sheetCornerOverlapPx,
        fittedRect(item).height * MAX_OVERLAP_HEIGHT_FRACTION,
    ).coerceAtLeast(0f)

    fun sheetTravelToRestPx(item: TimelineMediaProjection): Float = (baseTopPx(item) - sheetRestTopPx).coerceAtLeast(0f)

    fun photoTravelToRestPx(item: TimelineMediaProjection): Float = (sheetTravelToRestPx(item) - sheetOverlapPx(item)).coerceAtLeast(0f)

    fun maxSheetScrollPx(item: TimelineMediaProjection): Float {
        val containerH = viewerSize.height.toFloat()
        if (containerH <= 0f) return 0f
        val contentH = infoContentHeight(item.id).toFloat()
        if (contentH <= 0f) return 0f
        return (sheetRestTopPx + contentH - containerH).coerceAtLeast(0f)
    }

    /** The tile's ratio when known, else the ratio of the already-cached preview bitmap. */
    private fun resolvePhotoRatio(item: TimelineMediaProjection): Float {
        val baseRatio = item.ratio
        if (baseRatio.isFinite() && baseRatio > 0f && baseRatio != 1f) return baseRatio
        val cached = memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
        if (cached != null && cached.width > 0 && cached.height > 0) {
            return cached.width.toFloat() / cached.height.toFloat()
        }
        return if (baseRatio.isFinite() && baseRatio > 0f) baseRatio else 1f
    }
}

private const val SHEET_REST_TOP_FRACTION = 0.38f
private const val MAX_OVERLAP_HEIGHT_FRACTION = 0.5f

/** The photo's rect when fitted (contained and centered) in the viewer. */
internal fun computeFittedRect(containerWidth: Float, containerHeight: Float, ratio: Float): Rect {
    if (containerWidth <= 0f || containerHeight <= 0f) return Rect.Zero
    val safeRatio = if (ratio.isFinite() && ratio > 0f) ratio else 1f
    val containerRatio = containerWidth / containerHeight
    val fittedWidth: Float
    val fittedHeight: Float
    if (safeRatio > containerRatio) {
        fittedWidth = containerWidth
        fittedHeight = containerWidth / safeRatio
    } else {
        fittedHeight = containerHeight
        fittedWidth = containerHeight * safeRatio
    }
    val left = (containerWidth - fittedWidth) / 2f
    val top = (containerHeight - fittedHeight) / 2f
    return Rect(left, top, left + fittedWidth, top + fittedHeight)
}
