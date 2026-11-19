package dev.ruurd.photos.features.viewer

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntSize
import dev.ruurd.photos.features.viewer.components.HeroImageOverlay
import dev.ruurd.photos.features.viewer.components.VideoPlayBadge
import dev.ruurd.photos.features.viewer.components.ViewerChromeOverlay
import dev.ruurd.photos.features.viewer.components.ViewerPreSyncScrollEffect
import dev.ruurd.photos.features.viewer.components.ViewerPrefetchEffect
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.features.viewer.components.rememberMediaViewerTransitionState
import kotlinx.coroutines.flow.filterNotNull

/**
 * Full-screen media pager with the tile↔fullscreen hero transition, drag-to-dismiss and floating
 * chrome. All transition state lives in [rememberMediaViewerTransitionState]; the chrome, the hero
 * bitmaps and the viewer-side effects are separate composables.
 */
@Composable
fun MediaViewerScreen(
    viewModel: MediaViewerViewModel,
    baseUrl: String,
    openSourceRect: Rect?,
    containerSize: IntSize,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    onDismissFinished: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items

    val currentItems by rememberUpdatedState(items)
    val currentActiveId by rememberUpdatedState(uiState.activeMediaId)
    val currentOnHiddenChanged by rememberUpdatedState(onHiddenTimelineMediaChanged)

    val pagerState = key(uiState.isProjectionLoading) {
        val targetInitialIndex = if (uiState.isProjectionLoading) {
            0
        } else {
            uiState.initialIndex.coerceIn(0, (items.size - 1).coerceAtLeast(0))
        }
        rememberPagerState(
            initialPage = targetInitialIndex,
            pageCount = { currentItems.size },
        )
    }

    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f

    val transition = rememberMediaViewerTransitionState(
        pagerState = pagerState,
        openSourceRect = openSourceRect,
        containerSize = containerSize,
        items = items,
        activeMediaId = uiState.activeMediaId,
        isZoomed = isZoomed,
        isChromeVisible = uiState.isChromeVisible,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
        onPreSyncScroll = onPreSyncScroll,
        onDismissFinished = onDismissFinished,
    )

    LaunchedEffect(transition.hasValidOpenSource) {
        transition.playOpening()
    }

    // Hide the origin tile only once the opening hero actually has pixels to paint over it
    LaunchedEffect(transition.isOpeningHero, transition.openingHeroImageReady, items) {
        if (transition.isOpeningHero && transition.openingHeroImageReady && items.isNotEmpty()) {
            currentOnHiddenChanged(uiState.activeMediaId)
        }
    }

    LaunchedEffect(transition.closingState) {
        transition.playClosing()
    }

    // Dismiss automatically if the library becomes empty or active item no longer exists
    LaunchedEffect(uiState.isActiveItemMissing) {
        if (uiState.isActiveItemMissing && !transition.isClosing) {
            transition.startClosing(Offset.Zero, 1f, 1f)
        }
    }

    // Anchor HorizontalPager by mediaId across live Room updates
    LaunchedEffect(pagerState) {
        snapshotFlow {
            if (pagerState.isScrollInProgress) null else currentItems to currentActiveId
        }
            .filterNotNull()
            .collect { (latestItems, activeId) ->
                if (latestItems.isEmpty()) return@collect
                val targetIndex = latestItems.indexOfFirst { it.id == activeId }
                if (targetIndex >= 0 && pagerState.currentPage != targetIndex) {
                    pagerState.scrollToPage(targetIndex)
                }
            }
    }

    // Synchronize page changes with ViewModel activeMediaId
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.currentPage }.collect { page ->
            activePageZoomFraction = 0f
            val latestItems = currentItems
            if (page in latestItems.indices) {
                viewModel.onPageChanged(latestItems[page].id)
            }
        }
    }

    ViewerPreSyncScrollEffect(
        pagerState = pagerState,
        items = items,
        isClosing = { transition.isClosing },
        onPreSyncScroll = onPreSyncScroll,
    )

    ViewerPrefetchEffect(
        pagerState = pagerState,
        items = items,
        baseUrl = baseUrl,
    )

    BackHandler(enabled = !transition.isClosing) {
        transition.startClosing(Offset.Zero, 1f, transition.backdropAlpha)
    }

    // Outer Box stays untransformed so pointerInput reads stable screen coordinates during 2D drag
    Box(
        modifier = modifier
            .fillMaxSize()
            .onSizeChanged(transition::onSizeChanged)
            .then(transition.dismissDragModifier()),
    ) {
        // Backdrop scrim
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black.copy(alpha = transition.backdropAlpha)),
        )

        // Interactive Pager Layer (hidden only while ClosingHeroOverlay is flying back to the tile)
        if (transition.closingState == null && items.isNotEmpty()) {
            val isPagerScrollEnabled = !isZoomed && !transition.isDragging &&
                !transition.isOpeningHero

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer {
                        translationX = transition.dragOffset.x
                        translationY = transition.dragOffset.y
                        scaleX = transition.viewerScale
                        scaleY = transition.viewerScale
                        alpha = transition.pagerAlpha
                    },
            ) {
                HorizontalPager(
                    state = pagerState,
                    key = { page -> if (page in items.indices) items[page].id else page },
                    beyondViewportPageCount = 1,
                    userScrollEnabled = isPagerScrollEnabled,
                    modifier = Modifier.fillMaxSize(),
                ) { page ->
                    val item = items.getOrNull(page)
                    if (item != null) {
                        key(item.id) {
                            val isCurrentPage = pagerState.currentPage == page
                            val isSettledPage = pagerState.settledPage == page
                            Box(modifier = Modifier.fillMaxSize()) {
                                ZoomablePhotoPage(
                                    item = item,
                                    baseUrl = baseUrl,
                                    isActivePage = isCurrentPage,
                                    isSettledPage = isSettledPage,
                                    isOpeningTransition = transition.isOpeningHero,
                                    onTap = {
                                        if (!transition.isOpeningHero && !transition.isClosing) {
                                            viewModel.toggleChrome()
                                        }
                                    },
                                    onZoomFractionChanged = { fraction ->
                                        if (page == pagerState.currentPage) {
                                            activePageZoomFraction = fraction
                                        }
                                    },
                                    modifier = Modifier.fillMaxSize(),
                                )
                                // Phase 2C swaps this slot for a real VideoPlayerPage.
                                if (item.isVideo) {
                                    VideoPlayBadge(
                                        durationMs = item.durationMs,
                                        modifier = Modifier.align(Alignment.Center),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        // Opening Hero Overlay: animates smoothly from tile's boundsInRoot to centered fitted rect
        if (transition.isOpeningHero && openSourceRect != null) {
            val openingItem = items.firstOrNull { it.id == uiState.activeMediaId }
                ?: items.getOrNull(pagerState.currentPage)
            if (openingItem != null) {
                HeroImageOverlay(
                    item = openingItem,
                    baseUrl = baseUrl,
                    rect = transition.openingRect(openSourceRect, openingItem),
                    alpha = 1f,
                    onImageReady = transition::onHeroImageReady,
                )
            }
        }

        // Closing Hero Overlay: starts from exact dragged/released rect and lands directly on tile
        transition.closingState?.let { closing ->
            HeroImageOverlay(
                item = closing.item,
                baseUrl = baseUrl,
                rect = transition.closingRect(closing),
                alpha = transition.closingAlpha(closing),
            )
        }

        val chromeAlpha = transition.chromeAlpha(uiState.isChromeVisible)
        if (chromeAlpha > 0.01f) {
            ViewerChromeOverlay(
                alpha = chromeAlpha,
                items = items,
                activeMediaId = uiState.activeMediaId,
                currentPage = pagerState.currentPage,
                isProjectionLoading = uiState.isProjectionLoading,
                onBack = {
                    transition.startClosing(Offset.Zero, 1f, transition.backdropAlpha)
                },
            )
        }
    }
}
