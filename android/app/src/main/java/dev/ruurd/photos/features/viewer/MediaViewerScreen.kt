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
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
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
import dev.ruurd.photos.domain.model.ResolvedVideoSource
import dev.ruurd.photos.domain.model.VideoPlaybackPolicy
import dev.ruurd.photos.features.viewer.components.HeroImageOverlay
import dev.ruurd.photos.features.viewer.components.MediaInfoPanel
import dev.ruurd.photos.features.viewer.components.VideoPlayerPage
import dev.ruurd.photos.features.viewer.components.ViewerChromeOverlay
import dev.ruurd.photos.features.viewer.components.ViewerHdrCoordinator
import dev.ruurd.photos.features.viewer.components.ViewerPreSyncScrollEffect
import dev.ruurd.photos.features.viewer.components.ViewerPrefetchEffect
import dev.ruurd.photos.features.viewer.components.ViewerSystemBarsEffect
import dev.ruurd.photos.features.viewer.components.ViewerVideoChromeState
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.features.viewer.components.rememberMediaViewerTransitionState
import dev.ruurd.photos.features.viewer.components.rememberViewerSystemBars
import dev.ruurd.photos.features.viewer.components.rememberViewerVideoController
import dev.ruurd.photos.features.viewer.components.viewerVerticalGestures
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import okhttp3.OkHttpClient
import kotlin.math.abs
import kotlin.math.roundToInt

/**
 * Full-screen media pager with the tile↔fullscreen hero transition, drag-to-dismiss, floating
 * chrome, and the inline swipe-up Media Info panel anchored under each page so horizontal swipes
 * move the media item and its info sheet together.
 */
@Composable
fun MediaViewerScreen(
    viewModel: MediaViewerViewModel,
    okHttpClient: OkHttpClient,
    baseUrl: String,
    openSourceRect: Rect?,
    containerSize: IntSize,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    onDismissFinished: () -> Unit,
    modifier: Modifier = Modifier,
    openSourceCorners: TileCornerRadii = TileCornerRadii.Zero,
    isOnline: Boolean = true,
    onActiveMediaChanged: (String) -> Unit = {},
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items
    val context = LocalContext.current
    val actionScope = rememberCoroutineScope()
    val systemBars = rememberViewerSystemBars()
    val hdrCoordinator = remember(systemBars) { ViewerHdrCoordinator(systemBars) }

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

    val videoController = rememberViewerVideoController(
        okHttpClient = okHttpClient,
        isOnline = isOnline,
        preferences = uiState.videoPreferences,
        isChromeVisible = uiState.isChromeVisible,
        onLocalVideoFailed = viewModel::onLocalVideoFailed,
        onCloudFallbackTo480p = viewModel::onCloudVideoFallbackTo480p,
        onCloudVideoError = viewModel::onCloudVideoError,
    )

    val transition = rememberMediaViewerTransitionState(
        pagerState = pagerState,
        openSourceRect = openSourceRect,
        openSourceCorners = openSourceCorners,
        containerSize = containerSize,
        items = items,
        activeMediaId = uiState.activeMediaId,
        isZoomed = isZoomed,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
        onPreSyncScroll = onPreSyncScroll,
        onDismissFinished = onDismissFinished,
        onVerticalDragStart = videoController::pause,
        systemBars = systemBars,
    )

    ViewerSystemBarsEffect(
        systemBars = systemBars,
        isChromeVisible = uiState.isChromeVisible,
        isClosing = transition.isClosing,
        isInfoVisible = transition.shouldShowInfoPanel,
        isInfoCoveringStatusBar = transition.isInfoCoveringStatusBar,
        onHiddenTimelineMediaChanged = onHiddenTimelineMediaChanged,
    )

    val isHdrCoordinatorActive = !transition.isOpeningHero && !transition.isClosing
    SideEffect {
        hdrCoordinator.isViewerActive = isHdrCoordinatorActive
        if (isHdrCoordinatorActive) {
            hdrCoordinator.syncWindowColorMode()
        }
    }

    var wasOnline by remember { mutableStateOf(isOnline) }
    LaunchedEffect(isOnline) {
        if (isOnline && !wasOnline) {
            viewModel.retryActiveItemIfNeeded()
        }
        wasOnline = isOnline
    }

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
        if (transition.closingState != null) {
            videoController.stopAndClear()
        }
        transition.playClosing()
    }

    // Retry an "About" tap that arrived while zoomed or mid hero-open.
    LaunchedEffect(isZoomed, transition.isOpeningHero) {
        transition.retryPendingInfoRequest()
    }

    // Close the info panel cleanly if the user pinches or double-taps to zoom in
    LaunchedEffect(isZoomed) {
        if (isZoomed && transition.isInfoPanelActive) {
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

    // Synchronize page changes with ViewModel activeMediaId and stop video playback synchronously
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.currentPage }.collect { page ->
            activePageZoomFraction = 0f
            val latestItems = currentItems
            if (page in latestItems.indices) {
                val nextId = latestItems[page].id
                if (videoController.activeMediaId != null && videoController.activeMediaId != nextId) {
                    videoController.stopAndClear()
                }
                viewModel.onPageChanged(nextId)
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

    // ~130 ms after settling, trigger neighbor prefetch + cloud-only full-res download.
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
            .viewerVerticalGestures(transition),
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
                            val isOffscreen by remember(pagerState, page) {
                                derivedStateOf {
                                    (!pagerState.isScrollInProgress && pagerState.settledPage != page) ||
                                        abs(
                                            (page - pagerState.currentPage) -
                                                pagerState.currentPageOffsetFraction,
                                        ) >= 1f
                                }
                            }
                            val itemDetails = uiState.detailsById[item.id]
                                ?.takeIf { it.cloudId == item.cloudId }
                            val itemLocalInfo = uiState.localInfoById[item.id]

                            Box(modifier = Modifier.fillMaxSize()) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxSize()
                                        .graphicsLayer {
                                            translationY = -transition.pagePhotoTravelPx(item)
                                        },
                                ) {
                                    if (item.isVideo) {
                                        val isUnmetered = remember(
                                            item.id,
                                            uiState.videoPreferences.preferredQuality,
                                        ) {
                                            viewModel.isUnmeteredWithoutDataSaver()
                                        }
                                        val transientQuality = uiState.transientVideoQualityById[item.id]
                                        val localFailed = item.id in uiState.localVideoFallbackIds
                                        val isOriginalMissing = itemDetails?.isMissing == true ||
                                            item.id in uiState.videoUnavailableOriginalIds
                                        val resolvedSource = remember(
                                            item.localContentUri,
                                            localFailed,
                                            item.cloudId,
                                            baseUrl,
                                            item.hasThumbnails,
                                            isOriginalMissing,
                                            uiState.videoPreferences.preferredQuality,
                                            transientQuality,
                                            isUnmetered,
                                        ) {
                                            VideoPlaybackPolicy.resolveSource(
                                                localContentUri = item.localContentUri,
                                                localFailed = localFailed,
                                                cloudId = item.cloudId,
                                                baseUrl = baseUrl,
                                                hasThumbnails = item.hasThumbnails,
                                                isOriginalMissing = isOriginalMissing,
                                                savedQuality = uiState.videoPreferences.preferredQuality,
                                                transientQualityOverride = transientQuality,
                                                isUnmeteredWithoutDataSaver = isUnmetered,
                                            )
                                        }
                                        VideoPlayerPage(
                                            item = item,
                                            baseUrl = baseUrl,
                                            isSettledPage = isSettledPage,
                                            isClosing = transition.isClosing,
                                            isInfoPanelActive = transition.isInfoPanelActive,
                                            isDragging = transition.isDragging || transition.isDraggingInfo,
                                            videoController = videoController,
                                            resolvedSource = resolvedSource,
                                            errorReason = uiState.videoErrorById[item.id],
                                            onTap = {
                                                if (!transition.isOpeningHero && !transition.isClosing) {
                                                    if (transition.isInfoPanelActive) {
                                                        transition.requestCloseInfoPanel()
                                                    } else {
                                                        viewModel.toggleChrome()
                                                    }
                                                }
                                            },
                                            onRetry = { viewModel.retryVideoPlayback(item.id) },
                                            modifier = Modifier.fillMaxSize(),
                                        )
                                    } else {
                                        ZoomablePhotoPage(
                                            item = item,
                                            baseUrl = baseUrl,
                                            isActivePage = isCurrentPage,
                                            isSettledPage = isSettledPage,
                                            isOffscreen = isOffscreen,
                                            isOpeningTransition = transition.isOpeningHero,
                                            onTap = {
                                                if (!transition.isOpeningHero && !transition.isClosing) {
                                                    if (transition.isInfoPanelActive) {
                                                        transition.requestCloseInfoPanel()
                                                    } else {
                                                        viewModel.toggleChrome()
                                                    }
                                                }
                                            },
                                            fullResFile = uiState.fullResFiles[item.id],
                                            isInfoPanelRequested = isCurrentPage &&
                                                transition.isInfoPanelRequested,
                                            onFullResRequested = { viewModel.requestFullRes(item) },
                                            onZoomFractionChanged = { fraction ->
                                                if (page == pagerState.currentPage) {
                                                    activePageZoomFraction = fraction
                                                }
                                            },
                                            onHdrStateChanged = { hasPreviewGainmap, hasSubSamplingLayer ->
                                                hdrCoordinator.updatePage(
                                                    mediaId = item.id,
                                                    isVisible = !isOffscreen,
                                                    previewHasGainmap = hasPreviewGainmap,
                                                    hasSubSamplingLayer = hasSubSamplingLayer,
                                                )
                                            },
                                            onSyncWindowColorMode = hdrCoordinator::syncWindowColorMode,
                                            onPageDisposed = { hdrCoordinator.removePage(item.id) },
                                            modifier = Modifier.fillMaxSize(),
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
                                                isOnline = isOnline,
                                                isDetailsLoading = item.id in uiState.loadingDetailsIds,
                                                hasDetailsFailed = item.id in uiState.failedDetailsIds,
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
            val hasLocalOrExistingServer = activeItem != null &&
                (activeItem.localContentUri != null || activeDetails?.isMissing != true)
            val videoChromeState = if (activeItem?.isVideo == true) {
                val activeSourceHeight = remember(activeDetails, activeLocalInfo) {
                    val w = activeDetails?.width ?: activeLocalInfo?.width
                    val h = activeDetails?.height ?: activeLocalInfo?.height
                    if (w != null && h != null && w > 0 && h > 0) minOf(w, h) else null
                }
                val isOriginalMissing = activeDetails?.isMissing == true ||
                    activeItem.id in uiState.videoUnavailableOriginalIds
                val qualityOptions = remember(
                    activeItem.hasThumbnails,
                    isOriginalMissing,
                    activeSourceHeight,
                ) {
                    VideoPlaybackPolicy.availableQualities(
                        hasThumbnails = activeItem.hasThumbnails,
                        isOriginalMissing = isOriginalMissing,
                        sourceHeight = activeSourceHeight,
                    ).toImmutableList()
                }
                val isPlayingActive = videoController.activeMediaId == activeItem.id && videoController.isPlaying
                val activeDurationMs = if (videoController.activeMediaId == activeItem.id &&
                    videoController.durationMs > 0L
                ) {
                    videoController.durationMs
                } else {
                    (activeItem.durationMs ?: 0).toLong()
                }
                val isCloudPlayback = activeItem.localContentUri == null ||
                    activeItem.id in uiState.localVideoFallbackIds
                val activeQuality = if (isCloudPlayback && videoController.activeMediaId == activeItem.id) {
                    videoController.activeCloudQuality
                } else {
                    null
                }
                ViewerVideoChromeState(
                    isPlaying = isPlayingActive,
                    isMuted = uiState.videoPreferences.isMuted,
                    isLoopEnabled = uiState.videoPreferences.isLoopEnabled,
                    playbackSpeed = uiState.videoPreferences.playbackSpeed,
                    durationMs = activeDurationMs,
                    positionMsProvider = videoController.positionMsProvider,
                    bufferedMsProvider = videoController.bufferedPositionMsProvider,
                    activeQuality = activeQuality,
                    qualityOptions = qualityOptions,
                    showQualitySelector = isCloudPlayback && activeItem.hasThumbnails,
                    onTogglePlayPause = videoController::togglePlayPause,
                    onToggleMute = viewModel::toggleVideoMute,
                    onSeekTo = videoController::seekTo,
                    onSelectQuality = { quality -> viewModel.selectVideoQuality(activeItem.id, quality) },
                    onSelectSpeed = viewModel::setVideoPlaybackSpeed,
                    onToggleLoop = viewModel::toggleVideoLoop,
                )
            } else {
                null
            }
            ViewerChromeOverlay(
                alpha = chromeAlpha,
                backButtonAlpha = backButtonAlpha,
                isSharing = uiState.isSharing,
                isCopying = uiState.isCopying,
                currentItem = activeItem,
                currentDetails = activeDetails,
                currentLocalInfo = activeLocalInfo,
                isShareEnabled = hasLocalOrExistingServer,
                isCopyEnabled = hasLocalOrExistingServer,
                videoChromeState = videoChromeState,
                onBack = {
                    if (transition.isInfoPanelActive) {
                        transition.requestCloseInfoPanel()
                    } else {
                        transition.startClosing(Offset.Zero, 1f, transition.backdropAlpha)
                    }
                },
                onAbout = transition::requestOpenInfoPanel,
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
