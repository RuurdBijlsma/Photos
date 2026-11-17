package dev.ruurd.photos.features.timeline

import android.net.Uri
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.layout.LazyLayoutCacheWindow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.domain.model.MonthHeaderUiItem
import dev.ruurd.photos.domain.model.TimelineGridItemUiModel
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.domain.model.TimelineRowUiItem
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.features.timeline.components.TimelineScrubber
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlin.math.roundToInt

/**
 * Read-only justified timeline: a `LazyColumn` of typed month headers and precomputed photo rows, plus
 * the right-edge scrubber and floating scroll date pill. All geometry comes from
 * `CalculateTimelineLayoutUseCase`; nothing here measures, formats a date, or builds a URL per frame.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun TimelineScreen(
    viewModel: TimelineViewModel,
    baseUrl: String,
    listState: LazyListState = rememberLazyListState(
        cacheWindow = remember {
            LazyLayoutCacheWindow(aheadFraction = 0.75f, behindFraction = 0.4f)
        }
    ),
    contentPadding: PaddingValues = PaddingValues(),
    topBarHeight: Dp = 0.dp,
    topBarVisible: Boolean = true,
    isSyncing: Boolean = false,
    isBootstrapping: Boolean = false,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val layout = uiState.layout
    val density = LocalDensity.current

    var isScrubbing by remember { mutableStateOf(false) }
    var viewportHeightDp by remember { mutableFloatStateOf(0f) }

    LaunchedEffect(isBootstrapping) {
        viewModel.setBootstrapping(isBootstrapping)
    }

    // Keep the viewport pinned to index 0 when newer months are prepended while already at the top;
    // otherwise LazyColumn key-anchoring shifts the scroll position down to the previous top header.
    val currentLayout by rememberUpdatedState(layout)
    LaunchedEffect(listState) {
        var lastTopRowId: String? = null
        var atTopBeforeUpdate = true
        var pinnedTopRowId: String? = null

        snapshotFlow {
            Triple(
                currentLayout,
                listState.firstVisibleItemIndex,
                listState.firstVisibleItemScrollOffset
            )
        }.collect { (activeLayout, firstIndex, firstOffset) ->
            val topRowId = activeLayout.rows.firstOrNull()?.id
            val topChanged = topRowId != null && lastTopRowId != null && topRowId != lastTopRowId
            if (topChanged) {
                pinnedTopRowId = if (atTopBeforeUpdate) topRowId else null
            }
            if (listState.isScrollInProgress) {
                pinnedTopRowId = null
            }
            if (pinnedTopRowId != null && pinnedTopRowId == topRowId &&
                !listState.isScrollInProgress && (firstIndex != 0 || firstOffset != 0)
            ) {
                listState.scrollToItem(0)
                pinnedTopRowId = null
                atTopBeforeUpdate = true
            } else {
                atTopBeforeUpdate = firstIndex == 0 && firstOffset == 0
            }
            lastTopRowId = topRowId
        }
    }

    // Rows scroll under the floating top bar and bottom nav; the padding only offsets the content
    // edges so the first and last rows are reachable.
    val topPaddingDp = contentPadding.calculateTopPadding().value
    val bottomPaddingDp = contentPadding.calculateBottomPadding().value + EXTRA_BOTTOM_PADDING_DP

    Box(
        modifier = modifier
            .fillMaxSize()
            .semantics { testTagsAsResourceId = true }
            .onSizeChanged {
                viewModel.onViewportChanged(it.width, density.density)
                viewportHeightDp = with(density) { it.height.toDp().value }
            }
    ) {
        when {
            uiState.isLoading || isBootstrapping || (layout.isEmpty && isSyncing) -> {
                CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
            }

            layout.isEmpty -> {
                EmptyTimeline(modifier = Modifier.align(Alignment.Center))
            }

            else -> {
                LazyColumn(
                    state = listState,
                    modifier = Modifier
                        .fillMaxSize()
                        .testTag("timeline_list"),
                    contentPadding = PaddingValues(
                        top = topPaddingDp.dp,
                        bottom = bottomPaddingDp.dp
                    )
                ) {
                    items(
                        items = layout.rows,
                        key = { item -> item.id },
                        contentType = { item ->
                            if (item is MonthHeaderUiItem) "header" else "row"
                        }
                    ) { item ->
                        when (item) {
                            is MonthHeaderUiItem -> MonthHeader(item, layout.headerHeightDp)
                            is TimelineRowUiItem -> TimelineRow(item, baseUrl, isScrubbing)
                        }
                    }
                }

                FloatingDatePillOverlay(
                    layout = layout,
                    listState = listState,
                    topPadding = contentPadding.calculateTopPadding(),
                    topBarHeight = topBarHeight,
                    topBarVisible = topBarVisible,
                    modifier = Modifier.align(Alignment.TopCenter)
                )

                if (layout.totalHeightDp + topPaddingDp + bottomPaddingDp > viewportHeightDp) {
                    TimelineScrubber(
                        layout = layout,
                        listState = listState,
                        viewportHeightDp = viewportHeightDp,
                        topPaddingDp = topPaddingDp,
                        bottomPaddingDp = bottomPaddingDp,
                        onScrubbingChanged = { isScrubbing = it },
                        modifier = Modifier
                            .align(Alignment.CenterEnd)
                            .padding(top = topPaddingDp.dp, bottom = bottomPaddingDp.dp)
                            .testTag("timeline_scrubber")
                    )
                }
            }
        }
    }
}

/**
 * Owns the scroll-derived label and the top-bar-hide offset, so neither a date change nor the slide
 * animation recomposes `TimelineScreen` or the list.
 */
@Composable
private fun FloatingDatePillOverlay(
    layout: TimelineLayoutIndex,
    listState: LazyListState,
    topPadding: Dp,
    topBarHeight: Dp,
    topBarVisible: Boolean,
    modifier: Modifier = Modifier
) {
    val topBufferPx = with(LocalDensity.current) { DATE_PILL_TOP_BUFFER_DP.dp.roundToPx() }

    val datePillLabel by remember(layout, listState, topBufferPx) {
        derivedStateOf {
            val isScrolledPastTop = listState.firstVisibleItemIndex > 0 ||
                    listState.firstVisibleItemScrollOffset > topBufferPx
            if (!isScrolledPastTop || layout.isEmpty) {
                null
            } else {
                layout.dateLabelAt(listState.firstVisibleItemIndex)
            }
        }
    }

    val statusBarOnlyTopDp = (topPadding - topBarHeight).coerceAtLeast(0.dp)
    val datePillTopOffset = animateDpAsState(
        targetValue = if (topBarVisible) topPadding + 8.dp else statusBarOnlyTopDp + 8.dp,
        label = "date_pill_top_offset"
    )

    AnimatedVisibility(
        visible = datePillLabel != null,
        enter = fadeIn() + slideInVertically { -it / 2 },
        exit = fadeOut() + slideOutVertically { -it / 2 },
        // Layout-offset only: `.padding()` would relayout this subtree on every animation frame.
        modifier = modifier.offset { IntOffset(0, datePillTopOffset.value.roundToPx()) }
    ) {
        // The label turns null before the exit animation runs, so keep the last one to animate out.
        var lastLabel by remember { mutableStateOf("") }
        datePillLabel?.let { lastLabel = it }
        FloatingDatePill(label = lastLabel)
    }
}

@Composable
private fun FloatingDatePill(
    label: String,
    modifier: Modifier = Modifier
) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        tonalElevation = 6.dp,
        shadowElevation = 8.dp,
        modifier = modifier
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge.copy(
                fontFamily = MontserratFontFamily,
                fontWeight = FontWeight.SemiBold
            ),
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
        )
    }
}

@Composable
private fun MonthHeader(item: MonthHeaderUiItem, heightDp: Float) {
    // Baseline alignment does not use the row's fixed height, so the texts are wrapped in a Box that
    // bottom-aligns them; that keeps the gap above the month larger than the gap below it.
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(heightDp.dp)
            .padding(start = 24.dp, end = 24.dp, top = 16.dp, bottom = 10.dp),
        contentAlignment = Alignment.BottomStart
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(
                text = item.title,
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.alignByBaseline()
            )
            item.yearLabel?.let { year ->
                Text(
                    text = year,
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Medium),
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f),
                    modifier = Modifier.alignByBaseline()
                )
            }
        }
    }
}

@Composable
private fun TimelineRow(item: TimelineRowUiItem, baseUrl: String, isScrubbing: Boolean) {
    val shape = RoundedCornerShape(
        topStart = if (item.isFirstOfMonth) PhotosRadii.Card else 0.dp,
        topEnd = if (item.isFirstOfMonth) PhotosRadii.Card else 0.dp,
        bottomStart = if (item.isLastOfMonth) PhotosRadii.Card else 0.dp,
        bottomEnd = if (item.isLastOfMonth) PhotosRadii.Card else 0.dp
    )

    // The row gap is part of the row box so its height matches the precomputed offsets.
    Box(
        modifier = Modifier.padding(
            bottom = if (item.isLastOfMonth) 0.dp else ITEM_GAP_DP.dp
        )
    ) {
        Row(
            modifier = Modifier
                .height(item.heightDp.dp)
                .clip(shape),
            horizontalArrangement = Arrangement.spacedBy(ITEM_GAP_DP.dp)
        ) {
            item.items.forEach { gridItem ->
                TimelineTile(
                    item = gridItem,
                    heightDp = item.heightDp,
                    thumbnailTargetHeightPx = item.thumbnailTargetHeightPx,
                    baseUrl = baseUrl,
                    isScrubbing = isScrubbing
                )
            }
        }
    }
}

@Composable
private fun TimelineTile(
    item: TimelineGridItemUiModel,
    heightDp: Float,
    thumbnailTargetHeightPx: Int,
    baseUrl: String,
    isScrubbing: Boolean
) {
    val context = LocalContext.current
    val density = LocalDensity.current.density
    val widthPx = (item.widthDp * density).roundToInt().coerceAtLeast(1)
    val heightPx = (heightDp * density).roundToInt().coerceAtLeast(1)

    // Plain holder, not state: setting this from `onSuccess` must not recompose the tile.
    val loadState = remember(item, thumbnailTargetHeightPx, baseUrl, widthPx, heightPx) {
        TileLoadState()
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
        gateLoads
    ) {
        ImageRequest.Builder(context)
            .data(tileModel(item, baseUrl, thumbnailTargetHeightPx))
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
    ) {
        // Coil 3's default model equality ignores CachePolicy changes, so uncached tiles that were
        // skipped during scrubbing must re-mount once scrubbing stops.
        key(gateLoads) {
            AsyncImage(
                model = request,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                onSuccess = { loadState.succeeded = true },
                modifier = Modifier.fillMaxSize()
            )
        }

        if (item.isVideo) {
            Row(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(4.dp)
                    .background(
                        color = Color.Black.copy(alpha = 0.6f),
                        shape = RoundedCornerShape(PhotosRadii.TileOverlay)
                    )
                    .padding(horizontal = 6.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.PlayArrow,
                    contentDescription = null,
                    tint = Color.White,
                    modifier = Modifier.size(12.dp)
                )
                if (item.durationLabel != null) {
                    Spacer(modifier = Modifier.width(2.dp))
                    Text(
                        text = item.durationLabel,
                        style = MaterialTheme.typography.labelSmall,
                        color = Color.White
                    )
                }
            }
        }

        if (item.syncStatus == SyncStatus.LOCAL_ONLY) {
            Icon(
                imageVector = Icons.Default.CloudOff,
                contentDescription = "Not backed up",
                tint = Color.White,
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(4.dp)
                    .background(Color.Black.copy(alpha = 0.4f), CircleShape)
                    .padding(3.dp)
                    .size(11.dp)
            )
        }
    }
}

@Composable
private fun EmptyTimeline(modifier: Modifier = Modifier) {
    Box(modifier = modifier.padding(32.dp), contentAlignment = Alignment.Center) {
        Text(
            text = "No photos yet.\nBacked-up photos and camera folders will appear here.",
            style = MaterialTheme.typography.bodyMedium.copy(textAlign = TextAlign.Center),
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

private val ITEM_GAP_DP = CalculateTimelineLayoutUseCase.ITEM_GAP_DP
private const val EXTRA_BOTTOM_PADDING_DP = 24f
private const val DATE_PILL_TOP_BUFFER_DP = 180f

/** Mutable-without-recomposition flag for "this tile already rendered its image". */
private class TileLoadState {
    var succeeded = false
}

private fun tileModel(
    item: TimelineGridItemUiModel,
    baseUrl: String,
    thumbnailTargetHeightPx: Int
): Any? {
    val localUri = item.localContentUri?.let(Uri::parse)
    val cloudUrl = cloudThumbnailUrl(baseUrl, item, thumbnailTargetHeightPx)
    return if (localUri != null) {
        MediaStoreThumbnail(
            uri = localUri,
            cloudId = item.cloudId,
            cloudFallbackUrl = cloudUrl,
            isVideo = item.isVideo
        )
    } else {
        cloudUrl
    }
}

/**
 * Pre-generated AVIF thumbnails are served without auth; items whose thumbnails are not ready use the
 * on-demand endpoint, which needs the session cookie (sent by the shared OkHttp client).
 */
private fun cloudThumbnailUrl(
    baseUrl: String,
    item: TimelineGridItemUiModel,
    heightPx: Int
): String? {
    val cloudId = item.cloudId ?: return null
    if (baseUrl.isBlank()) return null
    return if (item.hasThumbnails) {
        "$baseUrl/thumbnails/$cloudId/${heightPx}p.avif"
    } else {
        "$baseUrl/api/photos/$cloudId/thumbnail?size=$heightPx"
    }
}