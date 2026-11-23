package dev.ruurd.photos.features.viewer

import android.content.Intent
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntSize
import dev.ruurd.photos.core.image.TileCornerRadii
import dev.ruurd.photos.features.viewer.components.HeroImageOverlay
import dev.ruurd.photos.features.viewer.components.MediaInfoPanel
import dev.ruurd.photos.features.viewer.components.VideoPlayBadge
import dev.ruurd.photos.features.viewer.components.ViewerChromeOverlay
import dev.ruurd.photos.features.viewer.components.ViewerPreSyncScrollEffect
import dev.ruurd.photos.features.viewer.components.ViewerPrefetchEffect
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.features.viewer.components.rememberMediaViewerTransitionState
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlin.collections.component1
import kotlin.collections.component2
import kotlin.collections.get
import kotlin.compareTo
import kotlin.math.roundToInt
import kotlin.text.get

/**
 * Full-screen media pager with the tile↔fullscreen hero transition, drag-to-dismiss, floating
 * chrome, and the inline swipe-up Media Info panel anchored under each page so horizontal swipes
 * move the media item and its info sheet together.
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
    openSourceCorners: TileCornerRadii = TileCornerRadii.Zero,
    onActiveMediaChanged: (String) -> Unit = {},
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items
    val context = LocalContext.current
    val actionScope = rememberCoroutineScope()

    val currentItems by rememberUpdatedState(items)
    val currentActiveId by rememberUpdatedState(uiState.activeMediaId)
    val currentOnHiddenChanged by rememberUpdatedState(onHiddenTimelineMediaChanged)
    val currentOnActiveMediaChanged by rememberUpdatedState(onActiveMediaChanged)

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

    val activeItem = items.firstOrNull { it.id == uiState.activeMediaId }
        ?: items.getOrNull(pagerState.currentPage)
    val activeDetails = activeItem?.let { item ->
        uiState.detailsById[item.id]?.takeIf { it.cloudId == item.cloudId }
    }
    val activeLocalInfo = activeItem?.let { item ->
        uiState.localInfoById[item.id]
    }

    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f

    val transition = rememberMediaViewerTransitionState(
        pagerState = pagerState,
        openSourceRect = openSourceRect,
        openSourceCorners = openSourceCorners,
        containerSize = containerSize,
        items = items,
        activeMediaId = uiState.activeMediaId,
        isZoomed = isZoomed,
        isChromeVisible = uiState.isChromeVisible,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
        onPreSyncScroll = onPreSyncScroll,
        onInfoOpenChanged = { open ->
            if (open) viewModel.openInfoSheet() else viewModel.closeInfoSheet()
        },
        onDismissFinished = onDismissFinished,
    )

    LaunchedEffect(uiState.activeMediaId) {
        currentOnActiveMediaChanged(uiState.activeMediaId)
    }

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

    // Sync programmatic info sheet opens/closes (e.g. "About" menu item) with transition state
    LaunchedEffect(uiState.isInfoSheetOpen) {
        if (uiState.isInfoSheetOpen) {
            transition.requestOpenInfoPanel()
        } else if (transition.isInfoPanelActive) {
            transition.requestCloseInfoPanel()
        }
    }

    // Close the info panel cleanly if the user pinches or double-taps to zoom in
    LaunchedEffect(isZoomed) {
        if (isZoomed && transition.isInfoPanelActive) {
            viewModel.closeInfoSheet()
            transition.requestCloseInfoPanel()
        }
    }

    // Dismiss automatically if the library becomes empty or active item no longer exists
    LaunchedEffect(uiState.isActiveItemMissing) {
        if (uiState.isActiveItemMissing && !transition.isClosing) {
            transition.resetInfoPanelImmediate()
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

    // Reset per-item sheet scroll overflow once a horizontal swipe finishes settling on a new page
    LaunchedEffect(pagerState) {
        snapshotFlow {
            if (pagerState.isScrollInProgress || currentItems.isEmpty()) {
                null
            } else {
                pagerState.settledPage
            }
        }
            .filterNotNull()
            .collect { page ->
                currentItems.getOrNull(page)?.let { settledItem ->
                    transition.onHorizontalPageSettled(settledItem.id)
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

    // ~130 ms after settling, load /item (plus neighbors) and the local/offline info fallback.
    LaunchedEffect(pagerState) {
        snapshotFlow { if (currentItems.isEmpty()) null else pagerState.settledPage }
            .filterNotNull()
            .collectLatest { page ->
                delay(SETTLED_DETAILS_DELAY_MS)
                currentItems.getOrNull(page)?.let { viewModel.onSettledPage(it.id) }
            }
    }

    // Keep enabled throughout the closing animation so rapid double-back taps don't leak underneath.
    BackHandler {
        if (transition.isInfoPanelActive) {
            viewModel.closeInfoSheet()
            transition.requestCloseInfoPanel()
        } else if (!transition.isClosing) {
            transition.startClosing(Offset.Zero, 1f, transition.backdropAlpha)
        }
    }

    val chromeVisibilityAlpha by animateFloatAsState(
        targetValue = if (uiState.isChromeVisible) 1f else 0f,
        animationSpec = tween(durationMillis = CHROME_FADE_MS),
        label = "chrome_visibility_alpha",
    )

    val infoPanelAlpha by animateFloatAsState(
        targetValue = if (transition.shouldShowInfoPanel) 1f else 0f,
        animationSpec = tween(durationMillis = INFO_PANEL_FADE_MS),
        label = "info_panel_alpha",
    )

    // Outer Box stays untransformed so pointerInput reads stable screen coordinates during gestures
    Box(
        modifier = modifier
            .fillMaxSize()
            .onSizeChanged(transition::onSizeChanged)
            .then(with(transition) { Modifier.dismissDragModifier() }),
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
                    !transition.isDraggingInfo && !transition.isOpeningHero

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
                            val itemDetails = uiState.detailsById[item.id]
                                ?.takeIf { it.cloudId == item.cloudId }
                            val itemLocalInfo = uiState.localInfoById[item.id]

                            Box(modifier = Modifier.fillMaxSize()) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxSize()
                                        .graphicsLayer {
                                            translationY = -transition.pageTravelPx(item)
                                        },
                                ) {
                                    ZoomablePhotoPage(
                                        item = item,
                                        baseUrl = baseUrl,
                                        isActivePage = isCurrentPage,
                                        isSettledPage = isSettledPage,
                                        isOpeningTransition = transition.isOpeningHero,
                                        onTap = {
                                            if (!transition.isOpeningHero && !transition.isClosing) {
                                                if (transition.isInfoPanelActive) {
                                                    viewModel.closeInfoSheet()
                                                    transition.requestCloseInfoPanel()
                                                } else {
                                                    viewModel.toggleChrome()
                                                }
                                            }
                                        },
                                        onZoomFractionChanged = { fraction ->
                                            if (page == pagerState.currentPage) {
                                                activePageZoomFraction = fraction
                                            }
                                        },
                                        fullResFile = uiState.fullResFiles[item.id],
                                        onFullResRequested = { viewModel.requestFullRes(item) },
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

                                // Inline Media Info panel anchored directly underneath each page's
                                // media item so horizontal swipes slide photo + info together.
                                if (transition.shouldShowInfoPanel || infoPanelAlpha > 0.005f) {
                                    Layout(
                                        content = {
                                            MediaInfoPanel(
                                                details = itemDetails,
                                                localInfo = itemLocalInfo,
                                                item = item,
                                                modifier = Modifier
                                                    .onSizeChanged {
                                                        transition.onInfoContentHeightChanged(
                                                            item.id,
                                                            it.height,
                                                        )
                                                    }
                                                    .graphicsLayer { alpha = infoPanelAlpha },
                                            )
                                        },
                                    ) { measurables, constraints ->
                                        val minSheetHeight = (
                                                constraints.maxHeight -
                                                        transition.sheetRestTopPx.roundToInt()
                                                ).coerceAtLeast(0)
                                        val childConstraints = constraints.copy(
                                            minHeight = minSheetHeight,
                                            maxHeight = Constraints.Infinity,
                                        )
                                        val placeable =
                                            measurables.first().measure(childConstraints)
                                        layout(constraints.maxWidth, constraints.maxHeight) {
                                            placeable.placeRelative(
                                                0,
                                                transition.pageInfoPanelTopPx(item).roundToInt(),
                                            )
                                        }
                                    }
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
                    cornerRadii = transition.openingCornerRadii(),
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
                cornerRadii = transition.closingCornerRadii(closing),
            )
        }

        val chromeAlpha = transition.chromeAlpha(chromeVisibilityAlpha)
        val backButtonAlpha = transition.backButtonAlpha(chromeVisibilityAlpha)
        if (chromeAlpha > 0.01f || backButtonAlpha > 0.01f) {
            ViewerChromeOverlay(
                alpha = chromeAlpha,
                backButtonAlpha = backButtonAlpha,
                currentItem = activeItem,
                currentDetails = activeDetails,
                currentLocalInfo = activeLocalInfo,
                isShareEnabled = activeItem != null && activeDetails?.isMissing != true,
                isCopyEnabled = activeDetails?.isMissing != true,
                onBack = {
                    if (transition.isInfoPanelActive) {
                        viewModel.closeInfoSheet()
                        transition.requestCloseInfoPanel()
                    } else {
                        transition.startClosing(Offset.Zero, 1f, transition.backdropAlpha)
                    }
                },
                onAbout = viewModel::openInfoSheet,
                onCopy = {
                    actionScope.launch {
                        val copied = viewModel.copyToClipboard()
                        if (!copied) {
                            Toast.makeText(
                                context,
                                "Can't copy this photo",
                                Toast.LENGTH_SHORT,
                            ).show()
                        }
                    }
                },
                onShare = {
                    actionScope.launch {
                        val intent = viewModel.shareIntent()
                        if (intent == null) {
                            Toast.makeText(context, "Can't share this item", Toast.LENGTH_SHORT)
                                .show()
                        } else {
                            context.startActivity(Intent.createChooser(intent, "Share"))
                        }
                    }
                },
            )
        }
    }
}

private const val CHROME_FADE_MS = 180
private const val INFO_PANEL_FADE_MS = 90
private const val SETTLED_DETAILS_DELAY_MS = 130L