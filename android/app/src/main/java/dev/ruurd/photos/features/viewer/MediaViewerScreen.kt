package dev.ruurd.photos.features.viewer

import android.app.Activity
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.lerp
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import coil3.ImageLoader
import coil3.compose.AsyncImage
import coil3.compose.asPainter
import coil3.request.ImageRequest
import coil3.size.Dimension
import coil3.size.Size as CoilSize
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.TimelineTileBoundsRegistry
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

private data class ClosingHeroState(
    val item: TimelineMediaProjection,
    val startRect: Rect,
    val targetRect: Rect?,
    val startBackdropAlpha: Float
)

@Composable
fun MediaViewerScreen(
    viewModel: MediaViewerViewModel,
    baseUrl: String,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    boundsRegistry: TimelineTileBoundsRegistry,
    openSourceRect: Rect?,
    containerSize: IntSize,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    onDismissFinished: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items
    val isDarkTheme = isSystemInDarkTheme()
    val context = LocalContext.current
    val viewConfig = LocalViewConfiguration.current
    val coroutineScope = rememberCoroutineScope()
    val density = LocalDensity.current

    val currentItems by rememberUpdatedState(items)
    val currentActiveId by rememberUpdatedState(uiState.activeMediaId)
    val currentOnHiddenChanged by rememberUpdatedState(onHiddenTimelineMediaChanged)
    val currentOnPreSyncScroll by rememberUpdatedState(onPreSyncScroll)
    val currentOnDismissFinished by rememberUpdatedState(onDismissFinished)

    var viewerSize by remember { mutableStateOf(containerSize) }
    val initialIndex = remember { uiState.initialIndex.coerceIn(0, (items.size - 1).coerceAtLeast(0)) }
    val pagerState = rememberPagerState(
        initialPage = initialIndex,
        pageCount = { currentItems.size }
    )

    // Opening hero animation state
    val hasValidOpenSource = openSourceRect != null && viewerSize.width > 0 && viewerSize.height > 0
    var hasFinishedOpening by remember { mutableStateOf(!hasValidOpenSource) }
    val openProgress = remember { Animatable(if (hasValidOpenSource) 0f else 1f) }
    val fallbackFadeIn = remember { Animatable(if (hasValidOpenSource) 1f else 0f) }
    var openingHeroImageReady by remember { mutableStateOf(false) }

    // Closing hero animation state
    var closingState by remember { mutableStateOf<ClosingHeroState?>(null) }
    val closeProgress = remember { Animatable(0f) }

    // 2D finger drag-to-dismiss state
    var isActivelyDragging by remember { mutableStateOf(false) }
    var rawDragOffset by remember { mutableStateOf(Offset.Zero) }
    val settleDragOffset = remember { Animatable(Offset.Zero, Offset.VectorConverter) }
    val settleJobHolder = remember { arrayOfNulls<Job>(1) }
    val currentDragOffset = if (isActivelyDragging) rawDragOffset else settleDragOffset.value

    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f
    val currentIsZoomed by rememberUpdatedState(isZoomed)

    val isOpeningHero = hasValidOpenSource && !hasFinishedOpening && closingState == null
    val isClosing = closingState != null
    val currentIsOpeningHero by rememberUpdatedState(isOpeningHero)
    val currentIsClosing by rememberUpdatedState(isClosing)

    // System bar appearance management (keep navigation bar transparent edge-to-edge for predictive back)
    val activity = context as? Activity
    val window = activity?.window
    val insetsController = remember(window) {
        window?.let { WindowCompat.getInsetsController(it, it.decorView) }
    }

    DisposableEffect(window, isDarkTheme) {
        insetsController?.apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }
        onDispose {
            currentOnHiddenChanged(null)
            insetsController?.apply {
                isAppearanceLightStatusBars = !isDarkTheme
                isAppearanceLightNavigationBars = !isDarkTheme
                show(WindowInsetsCompat.Type.statusBars())
            }
        }
    }

    LaunchedEffect(uiState.isChromeVisible, isClosing) {
        if (isClosing) {
            insetsController?.show(WindowInsetsCompat.Type.statusBars())
        } else if (uiState.isChromeVisible) {
            insetsController?.show(WindowInsetsCompat.Type.statusBars())
        } else {
            insetsController?.hide(WindowInsetsCompat.Type.statusBars())
        }
    }

    fun restoreSystemBarsForTimeline() {
        if (!isDarkTheme) {
            insetsController?.isAppearanceLightStatusBars = true
            insetsController?.isAppearanceLightNavigationBars = true
        }
        insetsController?.show(WindowInsetsCompat.Type.statusBars())
    }

    fun resolvePhotoRatio(item: TimelineMediaProjection): Float {
        val baseRatio = item.ratio
        if (baseRatio.isFinite() && baseRatio > 0f && baseRatio != 1f) return baseRatio
        val cached = memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
        if (cached != null && cached.width > 0 && cached.height > 0) {
            return cached.width.toFloat() / cached.height.toFloat()
        }
        return if (baseRatio.isFinite() && baseRatio > 0f) baseRatio else 1f
    }

    fun startClosing(
        releaseOffset: Offset,
        releaseScale: Float,
        releaseBackdropAlpha: Float
    ) {
        if (currentIsClosing) return
        settleJobHolder[0]?.cancel()
        settleJobHolder[0] = null

        val latestItems = currentItems
        val activeId = currentActiveId
        val activeItem = latestItems.getOrNull(pagerState.currentPage)?.takeIf { it.id == activeId }
            ?: latestItems.firstOrNull { it.id == activeId }
            ?: latestItems.getOrNull(pagerState.currentPage)

        if (activeItem == null || viewerSize.width <= 0 || viewerSize.height <= 0) {
            restoreSystemBarsForTimeline()
            currentOnHiddenChanged(null)
            currentOnDismissFinished()
            return
        }

        val w = viewerSize.width.toFloat()
        val h = viewerSize.height.toFloat()
        val fittedRect = computeFittedRect(w, h, resolvePhotoRatio(activeItem))

        val startRect: Rect
        val startAlpha: Float
        if (isOpeningHero && openSourceRect != null) {
            // Reversing mid-open starts right from the current interpolated hero rect.
            val t = openProgress.value
            startRect = lerp(openSourceRect, fittedRect, t)
            startAlpha = t.coerceIn(0f, 1f)
        } else {
            val centerX = w / 2f + releaseOffset.x
            val centerY = h / 2f + releaseOffset.y
            val startW = fittedRect.width * releaseScale
            val startH = fittedRect.height * releaseScale
            startRect = Rect(
                left = centerX - startW / 2f,
                top = centerY - startH / 2f,
                right = centerX + startW / 2f,
                bottom = centerY + startH / 2f
            )
            startAlpha = releaseBackdropAlpha
        }

        val targetRect = boundsRegistry.boundsInRoot(activeItem.id)
        currentOnPreSyncScroll(activeItem.id)
        if (targetRect != null) {
            currentOnHiddenChanged(activeItem.id)
        } else {
            currentOnHiddenChanged(null)
        }

        restoreSystemBarsForTimeline()
        isActivelyDragging = false
        closingState = ClosingHeroState(
            item = activeItem,
            startRect = startRect,
            targetRect = targetRect,
            startBackdropAlpha = startAlpha
        )
    }

    // Drive opening hero animation (or fade-in when opened without source bounds)
    LaunchedEffect(hasValidOpenSource) {
        if (hasValidOpenSource) {
            openProgress.animateTo(
                targetValue = 1f,
                animationSpec = spring(
                    dampingRatio = 0.85f,
                    stiffness = Spring.StiffnessMediumLow
                )
            )
            if (!currentIsClosing) {
                hasFinishedOpening = true
                currentOnHiddenChanged(null)
            }
        } else {
            fallbackFadeIn.animateTo(1f, tween(durationMillis = FALLBACK_FADE_MS))
        }
    }

    // Hide the origin tile only once the opening hero actually has pixels to paint over it
    LaunchedEffect(isOpeningHero, openingHeroImageReady, items) {
        if (isOpeningHero && openingHeroImageReady && items.isNotEmpty()) {
            currentOnHiddenChanged(uiState.activeMediaId)
        }
    }

    // Drive closing hero animation
    LaunchedEffect(closingState) {
        val activeClosing = closingState ?: return@LaunchedEffect
        closeProgress.snapTo(0f)
        if (activeClosing.targetRect != null) {
            closeProgress.animateTo(
                targetValue = 1f,
                animationSpec = spring(
                    dampingRatio = 0.88f,
                    stiffness = Spring.StiffnessMediumLow
                )
            )
        } else {
            closeProgress.animateTo(
                targetValue = 1f,
                animationSpec = tween(durationMillis = FALLBACK_FADE_MS)
            )
        }
        currentOnHiddenChanged(null)
        currentOnDismissFinished()
    }

    // Dismiss automatically if the library becomes empty or active item no longer exists
    LaunchedEffect(uiState.isActiveItemMissing) {
        if (uiState.isActiveItemMissing && !currentIsClosing) {
            startClosing(Offset.Zero, 1f, 1f)
        }
    }

    // Anchor HorizontalPager by mediaId across Loading -> Loaded transitions and live Room updates
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

    // Silent pre-sync of timeline scroll position ~250ms after settledPage pauses
    LaunchedEffect(pagerState) {
        snapshotFlow {
            if (currentItems.isEmpty()) null else pagerState.settledPage
        }
            .filterNotNull()
            .collectLatest { settledIndex ->
                delay(250L)
                val latestItems = currentItems
                if (settledIndex in latestItems.indices && !currentIsClosing) {
                    currentOnPreSyncScroll(latestItems[settledIndex].id)
                }
            }
    }

    // Neighbor prefetch for currentPage ± 1 previews after ~130ms settle delay
    LaunchedEffect(pagerState, baseUrl) {
        snapshotFlow {
            if (currentItems.isEmpty()) null else pagerState.settledPage to currentItems
        }
            .filterNotNull()
            .collectLatest { (settledIndex, latestItems) ->
                delay(130L)
                val neighbors = listOf(settledIndex - 1, settledIndex + 1)
                    .filter { it in latestItems.indices }
                for (idx in neighbors) {
                    val neighbor = latestItems[idx]
                    val localUri = neighbor.localContentUri?.let(Uri::parse)
                    val targetHeight = if (neighbor.isVideo) 720 else 1440
                    val cloudId = neighbor.cloudId?.takeIf { it.isNotBlank() }
                    val fallbackUrl = if (cloudId != null && baseUrl.isNotBlank()) {
                        if (neighbor.hasThumbnails) {
                            "$baseUrl/thumbnails/$cloudId/${targetHeight}p.avif"
                        } else {
                            "$baseUrl/api/photos/$cloudId/thumbnail?size=$targetHeight"
                        }
                    } else {
                        null
                    }
                    val previewModel = when {
                        localUri != null -> MediaStoreThumbnail(
                            uri = localUri,
                            cloudId = neighbor.cloudId,
                            cloudFallbackUrl = fallbackUrl,
                            isVideo = neighbor.isVideo,
                            bypassPlatformThumbnail = !neighbor.isVideo
                        )
                        else -> fallbackUrl
                    }
                    if (previewModel != null) {
                        val reqBuilder = ImageRequest.Builder(context).data(previewModel)
                        if (localUri != null) {
                            reqBuilder.size(targetHeight, targetHeight)
                        } else {
                            reqBuilder.size(CoilSize(Dimension.Undefined, Dimension.Pixels(targetHeight)))
                        }
                        imageLoader.enqueue(reqBuilder.build())
                    }
                }
            }
    }

    val dragDistanceY = currentDragOffset.y.coerceAtLeast(0f)
    val viewerScale = (1f - (dragDistanceY / 1400f)).coerceIn(0.55f, 1f)
    val dragBackdropAlpha = (1f - (dragDistanceY / 500f)).coerceIn(0f, 1f)
    val chromeDragAlpha = (1f - (dragDistanceY / 90f)).coerceIn(0f, 1f)

    val effectiveBackdropAlpha = when {
        closingState != null -> {
            val closing = closingState!!
            (closing.startBackdropAlpha * (1f - closeProgress.value)).coerceIn(0f, 1f)
        }
        isOpeningHero -> openProgress.value.coerceIn(0f, 1f)
        hasValidOpenSource -> dragBackdropAlpha
        else -> (fallbackFadeIn.value * dragBackdropAlpha).coerceIn(0f, 1f)
    }

    val effectiveChromeAlpha = when {
        closingState != null -> 0f
        !uiState.isChromeVisible -> 0f
        isOpeningHero -> ((openProgress.value * 1.4f) - 0.2f).coerceIn(0f, 1f)
        hasValidOpenSource -> chromeDragAlpha
        else -> (fallbackFadeIn.value * chromeDragAlpha).coerceIn(0f, 1f)
    }

    BackHandler(enabled = !isClosing) {
        startClosing(Offset.Zero, 1f, effectiveBackdropAlpha)
    }

    val dismissThresholdPx = with(density) { 110.dp.toPx() }
    val fastFlickMinDistancePx = with(density) { 40.dp.toPx() }

    // Outer Box stays untransformed so pointerInput reads stable screen coordinates during 2D drag
    Box(
        modifier = modifier
            .fillMaxSize()
            .onSizeChanged { viewerSize = it }
            .pointerInput(Unit) {
                val touchSlop = viewConfig.touchSlop
                val velocityTracker = VelocityTracker()

                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    if (currentIsClosing || currentIsOpeningHero || currentIsZoomed) {
                        return@awaitEachGesture
                    }
                    val pointerId = down.id
                    var totalDx = 0f
                    var totalDy = 0f
                    var isDraggingToDismiss = false
                    velocityTracker.resetTracking()
                    velocityTracker.addPosition(down.uptimeMillis, down.position)

                    try {
                        while (true) {
                            val pass = if (isDraggingToDismiss) {
                                PointerEventPass.Initial
                            } else {
                                PointerEventPass.Main
                            }
                            val event = awaitPointerEvent(pass)
                            val change = event.changes.firstOrNull { it.id == pointerId } ?: break

                            val multiTouch = event.changes.count { it.pressed } > 1
                            if (multiTouch) break
                            if (!isDraggingToDismiss && change.isConsumed) break

                            val dx = change.position.x - change.previousPosition.x
                            val dy = change.position.y - change.previousPosition.y
                            totalDx += dx
                            totalDy += dy

                            if (!isDraggingToDismiss) {
                                if (abs(totalDy) > touchSlop &&
                                    abs(totalDy) > abs(totalDx) * 1.25f &&
                                    totalDy > 0f
                                ) {
                                    isDraggingToDismiss = true
                                    settleJobHolder[0]?.cancel()
                                    settleJobHolder[0] = null
                                    val draggedId = currentItems.getOrNull(pagerState.currentPage)?.id
                                        ?: currentActiveId
                                    currentOnHiddenChanged(draggedId)
                                    rawDragOffset = settleDragOffset.value + Offset(totalDx, totalDy)
                                    isActivelyDragging = true
                                    velocityTracker.addPosition(change.uptimeMillis, change.position)
                                    change.consume()
                                } else if (abs(totalDx) > touchSlop || (abs(totalDy) > touchSlop && totalDy < 0f)) {
                                    // Horizontal swipe belongs to HorizontalPager; upward swipe is not dismiss.
                                    break
                                }
                            } else {
                                velocityTracker.addPosition(change.uptimeMillis, change.position)
                                change.consume()
                                rawDragOffset += Offset(dx, dy)
                            }

                            if (!change.pressed) break
                        }
                    } finally {
                        if (isDraggingToDismiss && !currentIsClosing) {
                            val releaseOffset = rawDragOffset
                            val releaseY = releaseOffset.y.coerceAtLeast(0f)
                            val velocityY = velocityTracker.calculateVelocity().y
                            val shouldDismiss = releaseY > dismissThresholdPx ||
                                    (velocityY > 1000f && releaseY > fastFlickMinDistancePx)

                            if (shouldDismiss) {
                                val releaseScale = (1f - (releaseY / 1400f)).coerceIn(0.55f, 1f)
                                val releaseAlpha = (1f - (releaseY / 500f)).coerceIn(0f, 1f)
                                startClosing(releaseOffset, releaseScale, releaseAlpha)
                            } else {
                                settleJobHolder[0]?.cancel()
                                settleJobHolder[0] = coroutineScope.launch {
                                    settleDragOffset.snapTo(releaseOffset)
                                    isActivelyDragging = false
                                    settleDragOffset.animateTo(
                                        targetValue = Offset.Zero,
                                        animationSpec = spring(
                                            dampingRatio = Spring.DampingRatioNoBouncy,
                                            stiffness = Spring.StiffnessMedium
                                        )
                                    )
                                    if (!isActivelyDragging && !currentIsClosing) {
                                        currentOnHiddenChanged(null)
                                    }
                                }
                            }
                        }
                    }
                }
            }
    ) {
        // Backdrop scrim
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black.copy(alpha = effectiveBackdropAlpha))
        )

        // Interactive Pager Layer (hidden only while ClosingHeroOverlay is flying back to the tile)
        if (closingState == null && items.isNotEmpty()) {
            val isPagerScrollEnabled = !isZoomed && !isActivelyDragging && !isOpeningHero

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer {
                        translationX = currentDragOffset.x
                        translationY = currentDragOffset.y
                        scaleX = viewerScale
                        scaleY = viewerScale
                        alpha = when {
                            isOpeningHero -> 0f
                            hasValidOpenSource -> 1f
                            else -> fallbackFadeIn.value
                        }
                    }
            ) {
                HorizontalPager(
                    state = pagerState,
                    key = { page -> if (page in items.indices) items[page].id else page },
                    beyondViewportPageCount = 1,
                    userScrollEnabled = isPagerScrollEnabled,
                    modifier = Modifier.fillMaxSize()
                ) { page ->
                    val item = items.getOrNull(page)
                    if (item != null) {
                        key(item.id) {
                            val isCurrentPage = pagerState.currentPage == page
                            val isSettledPage = pagerState.settledPage == page
                            ZoomablePhotoPage(
                                item = item,
                                baseUrl = baseUrl,
                                imageLoader = imageLoader,
                                memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                                isActivePage = isCurrentPage,
                                isSettledPage = isSettledPage,
                                isOpeningTransition = isOpeningHero,
                                onTap = {
                                    if (!isOpeningHero && !isClosing) {
                                        viewModel.toggleChrome()
                                    }
                                },
                                onZoomFractionChanged = { fraction ->
                                    if (page == pagerState.currentPage) {
                                        activePageZoomFraction = fraction
                                    }
                                },
                                modifier = Modifier.fillMaxSize()
                            )
                        }
                    }
                }
            }
        }

        // Opening Hero Overlay: animates smoothly from tile's boundsInRoot to centered fitted rect
        if (isOpeningHero && openSourceRect != null) {
            val openingItem = items.getOrNull(pagerState.currentPage)
                ?: items.firstOrNull { it.id == uiState.activeMediaId }
            if (openingItem != null) {
                val targetRect = computeFittedRect(
                    containerWidth = viewerSize.width.toFloat(),
                    containerHeight = viewerSize.height.toFloat(),
                    ratio = resolvePhotoRatio(openingItem)
                )
                val animatedRect = lerp(openSourceRect, targetRect, openProgress.value)
                HeroImageOverlay(
                    item = openingItem,
                    baseUrl = baseUrl,
                    rect = animatedRect,
                    alpha = 1f,
                    imageLoader = imageLoader,
                    memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                    onImageReady = { openingHeroImageReady = true }
                )
            }
        }

        // Closing Hero Overlay: starts from exact dragged/released rect and lands directly on tile
        val activeClosing = closingState
        if (activeClosing != null) {
            val progress = closeProgress.value
            val currentRect: Rect
            val heroAlpha: Float
            if (activeClosing.targetRect != null) {
                currentRect = lerp(activeClosing.startRect, activeClosing.targetRect, progress)
                heroAlpha = 1f
            } else {
                val shrinkScale = 1f - 0.15f * progress
                val cx = activeClosing.startRect.center.x
                val cy = activeClosing.startRect.center.y
                val w = activeClosing.startRect.width * shrinkScale
                val h = activeClosing.startRect.height * shrinkScale
                currentRect = Rect(
                    left = cx - w / 2f,
                    top = cy - h / 2f,
                    right = cx + w / 2f,
                    bottom = cy + h / 2f
                )
                heroAlpha = (1f - progress).coerceIn(0f, 1f)
            }

            HeroImageOverlay(
                item = activeClosing.item,
                baseUrl = baseUrl,
                rect = currentRect,
                alpha = heroAlpha,
                imageLoader = imageLoader,
                memoryCacheKeyRegistry = memoryCacheKeyRegistry
            )
        }

        // Floating Viewer Chrome (anchored to screen edges, fades cleanly during open & drag)
        if (effectiveChromeAlpha > 0.01f) {
            val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
            val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { alpha = effectiveChromeAlpha }
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = statusBarPadding + 6.dp, start = 8.dp, end = 8.dp)
                ) {
                    ViewerCircleButton(
                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Back",
                        onClick = { startClosing(Offset.Zero, 1f, effectiveBackdropAlpha) }
                    )

                    Spacer(modifier = Modifier.width(8.dp))

                    Surface(
                        shape = CircleShape,
                        color = ViewerPillColor,
                        border = ViewerPillBorder
                    ) {
                        val counterText = if (items.isNotEmpty()) {
                            val displayIndex = (pagerState.currentPage + 1).coerceIn(1, items.size)
                            "$displayIndex of ${items.size}"
                        } else {
                            ""
                        }
                        Text(
                            text = counterText,
                            style = MaterialTheme.typography.titleSmall.copy(
                                fontFamily = MontserratFontFamily,
                                fontWeight = FontWeight.SemiBold
                            ),
                            color = Color.White,
                            modifier = Modifier.padding(horizontal = 14.dp, vertical = 9.dp)
                        )
                    }

                    Spacer(modifier = Modifier.weight(1f))

                    var isMenuExpanded by remember { mutableStateOf(false) }
                    Box {
                        ViewerCircleButton(
                            imageVector = Icons.Default.MoreVert,
                            contentDescription = "Options",
                            onClick = { isMenuExpanded = true }
                        )

                        val currentItem = items.getOrNull(pagerState.currentPage)
                        DropdownMenu(
                            expanded = isMenuExpanded,
                            onDismissRequest = { isMenuExpanded = false }
                        ) {
                            DropdownMenuItem(
                                text = { Text("About") },
                                onClick = { isMenuExpanded = false }
                            )
                            if (currentItem != null && !currentItem.isVideo) {
                                DropdownMenuItem(
                                    text = { Text("Copy") },
                                    onClick = { isMenuExpanded = false }
                                )
                                DropdownMenuItem(
                                    text = { Text("Rotate clockwise") },
                                    onClick = { isMenuExpanded = false }
                                )
                            }
                            if (currentItem?.cloudId != null) {
                                DropdownMenuItem(
                                    text = { Text("Reprocess") },
                                    onClick = { isMenuExpanded = false }
                                )
                            }
                        }
                    }
                }

                Surface(
                    shape = CircleShape,
                    color = ViewerPillColor,
                    border = ViewerPillBorder,
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = navBarPadding + 14.dp)
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        ViewerPillAction(imageVector = Icons.Default.Share, label = "Share")
                        ViewerPillAction(imageVector = Icons.Default.Add, label = "Add to")
                        ViewerPillAction(imageVector = Icons.Default.Delete, label = "Bin")
                    }
                }
            }
        }
    }
}

@Composable
private fun HeroImageOverlay(
    item: TimelineMediaProjection,
    baseUrl: String,
    rect: Rect,
    alpha: Float,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onImageReady: () -> Unit = {}
) {
    val context = LocalContext.current

    val cachedPainter = remember(item.id, memoryCacheKeyRegistry, imageLoader, context) {
        memoryCacheKeyRegistry.get(item.id)
            ?.let { key -> imageLoader.memoryCache?.get(key)?.image }
            ?.asPainter(context)
    }

    LaunchedEffect(item.id, cachedPainter) {
        if (cachedPainter != null) {
            onImageReady()
        }
    }

    val previewModel = remember(item, baseUrl) {
        val localUri = item.localContentUri?.let(Uri::parse)
        val targetHeight = if (item.isVideo) 720 else 1440
        val cloudUrl = item.cloudId?.takeIf { it.isNotBlank() }?.let { cloudId ->
            if (baseUrl.isBlank()) {
                null
            } else if (item.hasThumbnails) {
                "$baseUrl/thumbnails/$cloudId/${targetHeight}p.avif"
            } else {
                "$baseUrl/api/photos/$cloudId/thumbnail?size=$targetHeight"
            }
        }
        if (localUri != null) {
            MediaStoreThumbnail(
                uri = localUri,
                cloudId = item.cloudId,
                cloudFallbackUrl = cloudUrl,
                isVideo = item.isVideo,
                bypassPlatformThumbnail = !item.isVideo
            )
        } else {
            cloudUrl
        }
    }

    Box(
        modifier = Modifier
            .offset { IntOffset(rect.left.roundToInt(), rect.top.roundToInt()) }
            .layout { measurable, _ ->
                val w = rect.width.roundToInt().coerceAtLeast(1)
                val h = rect.height.roundToInt().coerceAtLeast(1)
                val placeable = measurable.measure(Constraints.fixed(w, h))
                layout(w, h) {
                    placeable.place(0, 0)
                }
            }
            .graphicsLayer { this.alpha = alpha }
            .clip(RectangleShape)
    ) {
        if (cachedPainter != null) {
            Image(
                painter = cachedPainter,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize()
            )
        } else if (previewModel != null) {
            val targetHeight = if (item.isVideo) 720 else 1440
            val request = remember(item.id, previewModel, targetHeight) {
                ImageRequest.Builder(context)
                    .data(previewModel)
                    .apply {
                        if (previewModel is MediaStoreThumbnail) {
                            size(targetHeight, targetHeight)
                        } else {
                            size(CoilSize(Dimension.Undefined, Dimension.Pixels(targetHeight)))
                        }
                    }
                    .build()
            }
            AsyncImage(
                model = request,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                onSuccess = { result ->
                    result.result.memoryCacheKey?.let { key ->
                        memoryCacheKeyRegistry.put(item.id, key)
                    }
                    onImageReady()
                },
                modifier = Modifier.fillMaxSize()
            )
        }
    }
}

private fun computeFittedRect(containerWidth: Float, containerHeight: Float, ratio: Float): Rect {
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

private const val FALLBACK_FADE_MS = 180

/** Dark translucent scrim with a hairline highlight, matching the web viewer's pills. */
private val ViewerPillColor = Color.Black.copy(alpha = 0.55f)
private val ViewerPillBorder = BorderStroke(1.dp, Color.White.copy(alpha = 0.12f))

@Composable
private fun ViewerCircleButton(
    imageVector: ImageVector,
    contentDescription: String,
    onClick: () -> Unit
) {
    Box(
        modifier = Modifier
            .size(40.dp)
            .clip(CircleShape)
            .background(ViewerPillColor)
            .border(1.dp, Color.White.copy(alpha = 0.12f), CircleShape)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center
    ) {
        Icon(
            imageVector = imageVector,
            contentDescription = contentDescription,
            tint = Color.White,
            modifier = Modifier.size(20.dp)
        )
    }
}

@Composable
private fun ViewerPillAction(imageVector: ImageVector, label: String) {
    Row(
        modifier = Modifier
            .clip(CircleShape)
            .padding(horizontal = 10.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = imageVector,
            contentDescription = label,
            tint = Color.White,
            modifier = Modifier.size(18.dp)
        )
        Spacer(modifier = Modifier.width(6.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = Color.White
        )
    }
}