package dev.ruurd.photos.features.timeline

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.domain.model.MonthHeaderUiItem
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.domain.model.TimelineRowUiItem
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.features.main.components.PullToRefreshState
import dev.ruurd.photos.features.main.components.pullToRefresh
import dev.ruurd.photos.features.timeline.components.TimelineScrubber
import dev.ruurd.photos.features.timeline.components.TimelineTile
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import dev.ruurd.photos.ui.theme.PhotosRadii

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
    listState: LazyListState,
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(),
    topBarHeight: Dp = 0.dp,
    topBarVisible: Boolean = true,
    isSyncing: Boolean = false,
    pullToRefreshState: PullToRefreshState? = null,
    hiddenMediaId: String? = null,
    onMediaClick: (
        mediaId: String,
        sourceRect: Rect?,
        sourceCorners: TileCornerRadii,
    ) -> Unit = { _, _, _ -> },
) {
    val uiState by viewModel.uiState.collectAsState()
    val layout = uiState.layout
    val density = LocalDensity.current

    var isScrubbing by remember { mutableStateOf(false) }
    var viewportHeightDp by remember { mutableFloatStateOf(0f) }

    DisposableEffect(pullToRefreshState) {
        onDispose { pullToRefreshState?.reset() }
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
                listState.firstVisibleItemScrollOffset,
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

    val isEmptyState = !uiState.isLoading && layout.isEmpty && !isSyncing

    // Rows scroll under the floating top bar and bottom nav; the padding only offsets the content
    // edges so the first and last rows are reachable.
    val topPaddingDp = contentPadding.calculateTopPadding().value
    val bottomPaddingDp = contentPadding.calculateBottomPadding().value + EXTRA_BOTTOM_PADDING_DP

    Box(
        modifier = modifier
            .fillMaxSize()
            .pullToRefresh(
                state = pullToRefreshState,
                listState = listState,
                isEmpty = isEmptyState,
                isScrubbing = isScrubbing,
            )
            .semantics { testTagsAsResourceId = true }
            .onSizeChanged {
                viewModel.onViewportChanged(it.width, density.density)
                viewportHeightDp = with(density) { it.height.toDp().value }
            },
    ) {
        when {
            uiState.isLoading || (layout.isEmpty && isSyncing) -> {
                CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
            }

            layout.isEmpty -> {
                EmptyTimeline(
                    contentPadding = contentPadding,
                    pullToRefreshState = pullToRefreshState,
                    modifier = Modifier.fillMaxSize(),
                )
            }

            else -> {
                LazyColumn(
                    state = listState,
                    modifier = Modifier
                        .fillMaxSize()
                        .graphicsLayer {
                            val offset = pullToRefreshState?.contentOffsetPx ?: 0f
                            if (offset != 0f) {
                                translationY = offset
                            }
                        }
                        .testTag("timeline_list"),
                    contentPadding = PaddingValues(
                        top = topPaddingDp.dp,
                        bottom = bottomPaddingDp.dp,
                    ),
                ) {
                    itemsIndexed(
                        items = layout.rows,
                        key = { _, item -> item.id },
                        contentType = { _, item ->
                            if (item is MonthHeaderUiItem) "header" else "row"
                        },
                    ) { index, item ->
                        val cascadeModifier = if (pullToRefreshState != null && index < CASCADE_ROW_LIMIT) {
                            Modifier.pullCascadeGraphicsLayer(
                                index = index,
                                pullState = pullToRefreshState,
                                scaleRows = item is TimelineRowUiItem,
                            )
                        } else {
                            Modifier
                        }
                        when (item) {
                            is MonthHeaderUiItem -> MonthHeader(
                                item = item,
                                heightDp = layout.headerHeightDp,
                                modifier = cascadeModifier,
                            )

                            is TimelineRowUiItem -> TimelineRow(
                                item = item,
                                baseUrl = baseUrl,
                                isScrubbing = isScrubbing,
                                hiddenMediaId = hiddenMediaId,
                                onMediaClick = onMediaClick,
                                modifier = cascadeModifier,
                            )
                        }
                    }
                }

                FloatingDatePillOverlay(
                    layout = layout,
                    listState = listState,
                    topPadding = contentPadding.calculateTopPadding(),
                    topBarHeight = topBarHeight,
                    topBarVisible = topBarVisible,
                    modifier = Modifier.align(Alignment.TopCenter),
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
                            .testTag("timeline_scrubber"),
                    )
                }
            }
        }
    }
}

private fun Modifier.pullCascadeGraphicsLayer(
    index: Int,
    pullState: PullToRefreshState,
    scaleRows: Boolean,
): Modifier = graphicsLayer {
    val progress = pullState.pullProgress
    if (progress > 0f) {
        val clamped = progress.coerceAtMost(1.35f)
        translationY = index * CASCADE_STEP_DP.dp.toPx() * clamped
        if (scaleRows) {
            val scale = 1f - 0.015f * clamped.coerceAtMost(1.1f)
            scaleX = scale
            scaleY = scale
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
    modifier: Modifier = Modifier,
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
        label = "date_pill_top_offset",
    )

    AnimatedVisibility(
        visible = datePillLabel != null,
        enter = fadeIn() + slideInVertically { -it / 2 },
        exit = fadeOut() + slideOutVertically { -it / 2 },
        // Layout-offset only: `.padding()` would relayout this subtree on every animation frame.
        modifier = modifier.offset { IntOffset(0, datePillTopOffset.value.roundToPx()) },
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
    modifier: Modifier = Modifier,
) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        tonalElevation = 6.dp,
        shadowElevation = 8.dp,
        modifier = modifier,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge.copy(
                fontFamily = MontserratFontFamily,
                fontWeight = FontWeight.SemiBold,
            ),
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )
    }
}

@Composable
private fun MonthHeader(
    item: MonthHeaderUiItem,
    heightDp: Float,
    modifier: Modifier = Modifier,
) {
    // Baseline alignment does not use the row's fixed height, so the texts are wrapped in a Box that
    // bottom-aligns them; that keeps the gap above the month larger than the gap below it.
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(heightDp.dp)
            .padding(start = 24.dp, end = 24.dp, top = 16.dp, bottom = 10.dp),
        contentAlignment = Alignment.BottomStart,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(
                text = item.title,
                style = MaterialTheme.typography.headlineMedium,
                color = MaterialTheme.colorScheme.onBackground,
                modifier = Modifier.alignByBaseline(),
            )
            item.yearLabel?.let { year ->
                Text(
                    text = year,
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Medium),
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.7f),
                    modifier = Modifier.alignByBaseline(),
                )
            }
        }
    }
}

@Composable
private fun TimelineRow(
    item: TimelineRowUiItem,
    baseUrl: String,
    isScrubbing: Boolean,
    hiddenMediaId: String?,
    onMediaClick: (mediaId: String, sourceRect: Rect?, sourceCorners: TileCornerRadii) -> Unit,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(
        topStart = if (item.isFirstOfMonth) PhotosRadii.Card else 0.dp,
        topEnd = if (item.isFirstOfMonth) PhotosRadii.Card else 0.dp,
        bottomStart = if (item.isLastOfMonth) PhotosRadii.Card else 0.dp,
        bottomEnd = if (item.isLastOfMonth) PhotosRadii.Card else 0.dp,
    )
    val cornerRadiusDp = PhotosRadii.Card.value

    // The row gap is part of the row box so its height matches the precomputed offsets.
    Box(
        modifier = modifier.padding(
            bottom = if (item.isLastOfMonth) 0.dp else ITEM_GAP_DP.dp,
        ),
    ) {
        Row(
            modifier = Modifier
                .height(item.heightDp.dp)
                .clip(shape),
            horizontalArrangement = Arrangement.spacedBy(ITEM_GAP_DP.dp),
        ) {
            item.items.forEachIndexed { index, gridItem ->
                val isFirstInRow = index == 0
                val isLastInRow = index == item.items.lastIndex
                val cornerRadii = remember(
                    item.isFirstOfMonth,
                    item.isLastOfMonth,
                    isFirstInRow,
                    isLastInRow,
                    cornerRadiusDp,
                ) {
                    TileCornerRadii(
                        topStartDp = if (item.isFirstOfMonth && isFirstInRow) cornerRadiusDp else 0f,
                        topEndDp = if (item.isFirstOfMonth && isLastInRow) cornerRadiusDp else 0f,
                        bottomEndDp = if (item.isLastOfMonth && isLastInRow) cornerRadiusDp else 0f,
                        bottomStartDp = if (item.isLastOfMonth && isFirstInRow) cornerRadiusDp else 0f,
                    )
                }
                TimelineTile(
                    item = gridItem,
                    heightDp = item.heightDp,
                    thumbnailTargetHeightPx = item.thumbnailTargetHeightPx,
                    baseUrl = baseUrl,
                    isScrubbing = isScrubbing,
                    isHidden = hiddenMediaId == gridItem.id,
                    cornerRadii = cornerRadii,
                    onClick = { rect -> onMediaClick(gridItem.id, rect, cornerRadii) },
                )
            }
        }
    }
}

@Composable
private fun EmptyTimeline(
    contentPadding: PaddingValues,
    pullToRefreshState: PullToRefreshState?,
    modifier: Modifier = Modifier,
) {
    val emptyListState = rememberLazyListState()
    LazyColumn(
        state = emptyListState,
        contentPadding = contentPadding,
        modifier = modifier.graphicsLayer {
            val offset = pullToRefreshState?.contentOffsetPx ?: 0f
            if (offset != 0f) {
                translationY = offset
            }
        },
    ) {
        item {
            Box(
                modifier = Modifier
                    .fillParentMaxSize()
                    .padding(32.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "No photos yet.\nBacked-up photos and camera folders will appear here.",
                    style = MaterialTheme.typography.bodyMedium.copy(textAlign = TextAlign.Center),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

private val ITEM_GAP_DP = CalculateTimelineLayoutUseCase.ITEM_GAP_DP
private const val EXTRA_BOTTOM_PADDING_DP = 24f
private const val DATE_PILL_TOP_BUFFER_DP = 180f
private const val CASCADE_ROW_LIMIT = 8
private const val CASCADE_STEP_DP = 3f
