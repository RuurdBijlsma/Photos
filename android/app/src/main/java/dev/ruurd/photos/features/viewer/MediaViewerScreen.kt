package dev.ruurd.photos.features.viewer

import android.app.Activity
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.ExperimentalSharedTransitionApi
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import coil3.ImageLoader
import coil3.request.ImageRequest
import coil3.size.Dimension
import coil3.size.Size as CoilSize
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlin.math.abs

@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
fun MediaViewerScreen(
    viewModel: MediaViewerViewModel,
    baseUrl: String,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    isViewerVisible: Boolean,
    allowSharedBoundsAnimation: Boolean,
    sharedTransitionScope: SharedTransitionScope?,
    onActiveMediaChanged: (mediaId: String) -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    onRequestDismiss: (activeMediaId: String) -> Unit,
    onDismissFinished: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items
    val isDarkTheme = isSystemInDarkTheme()
    val context = LocalContext.current
    val viewConfig = LocalViewConfiguration.current
    val coroutineScope = rememberCoroutineScope()

    val currentItems by rememberUpdatedState(items)
    val currentActiveId by rememberUpdatedState(uiState.activeMediaId)
    val currentIsViewerVisible by rememberUpdatedState(isViewerVisible)
    val currentOnActiveMediaChanged by rememberUpdatedState(onActiveMediaChanged)
    val currentOnPreSyncScroll by rememberUpdatedState(onPreSyncScroll)
    val currentOnRequestDismiss by rememberUpdatedState(onRequestDismiss)
    val currentOnDismissFinished by rememberUpdatedState(onDismissFinished)

    val initialIndex = remember { uiState.initialIndex.coerceIn(0, (items.size - 1).coerceAtLeast(0)) }
    val pagerState = rememberPagerState(
        initialPage = initialIndex,
        pageCount = { currentItems.size }
    )

    // Dismiss automatically if the library becomes empty or active item no longer exists
    LaunchedEffect(uiState.isActiveItemMissing) {
        if (uiState.isActiveItemMissing && currentIsViewerVisible) {
            currentOnRequestDismiss(currentActiveId)
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

    // Track drag Y synchronously during touch, then hand off to Animatable on release
    var isActivelyDragging by remember { mutableStateOf(false) }
    var rawDragY by remember { mutableFloatStateOf(0f) }
    val settleDragY = remember { Animatable(0f) }
    val currentDragY = if (isActivelyDragging) rawDragY else settleDragY.value

    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f
    val currentIsZoomed by rememberUpdatedState(isZoomed)

    // Backdrop fade for enter and exit transitions
    val backdropReveal = remember { Animatable(if (isViewerVisible) 0f else 1f) }
    LaunchedEffect(isViewerVisible) {
        if (isViewerVisible) {
            backdropReveal.animateTo(1f, tween(durationMillis = BACKDROP_FADE_MS))
        } else {
            // Let the visible=false frame commit before checking SharedTransitionScope.isTransitionActive
            withFrameNanos { }
            backdropReveal.animateTo(0f, tween(durationMillis = BACKDROP_FADE_MS))
            if (sharedTransitionScope != null) {
                snapshotFlow { !sharedTransitionScope.isTransitionActive }.first { it }
            }
            currentOnDismissFinished()
        }
    }

    // Synchronize page changes with ViewModel activeMediaId and host shared-element key
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.currentPage }.collect { page ->
            activePageZoomFraction = 0f
            val latestItems = currentItems
            if (page in latestItems.indices) {
                val activeId = latestItems[page].id
                viewModel.onPageChanged(activeId)
                currentOnActiveMediaChanged(activeId)
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
                if (settledIndex in latestItems.indices) {
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
                            isVideo = neighbor.isVideo
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

    // System bar appearance management:
    // Keep navigation bar transparent edge-to-edge so predictive back stays active.
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
            insetsController?.apply {
                isAppearanceLightStatusBars = !isDarkTheme
                isAppearanceLightNavigationBars = !isDarkTheme
                show(WindowInsetsCompat.Type.statusBars())
            }
        }
    }

    LaunchedEffect(uiState.isChromeVisible, isViewerVisible) {
        if (!isViewerVisible) {
            insetsController?.show(WindowInsetsCompat.Type.statusBars())
        } else if (uiState.isChromeVisible) {
            insetsController?.show(WindowInsetsCompat.Type.statusBars())
        } else {
            insetsController?.hide(WindowInsetsCompat.Type.statusBars())
        }
    }

    fun triggerDismiss() {
        if (!isDarkTheme) {
            insetsController?.isAppearanceLightStatusBars = true
            insetsController?.isAppearanceLightNavigationBars = true
        }
        currentOnRequestDismiss(currentActiveId)
    }

    BackHandler(enabled = isViewerVisible) {
        triggerDismiss()
    }

    val dragBackdropFactor = (1f - (currentDragY / 600f)).coerceIn(0f, 1f)
    val effectiveBackdropAlpha = (backdropReveal.value * dragBackdropFactor).coerceIn(0f, 1f)
    val viewerScale = (1f - (currentDragY / 1800f)).coerceIn(0.85f, 1f)

    val dismissThresholdPx = with(LocalDensity.current) { 120.dp.toPx() }

    // Outer Box stays untransformed so pointerInput reads stable screen coordinates during drag
    Box(
        modifier = modifier
            .fillMaxSize()
            .pointerInput(Unit) {
                val touchSlop = viewConfig.touchSlop

                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    if (!currentIsViewerVisible || currentIsZoomed) return@awaitEachGesture
                    val pointerId = down.id
                    var totalDx = 0f
                    var totalDy = 0f
                    var isDraggingDown = false

                    try {
                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull { it.id == pointerId } ?: break

                            if (change.isConsumed || event.changes.size > 1) break

                            val dx = change.position.x - change.previousPosition.x
                            val dy = change.position.y - change.previousPosition.y
                            totalDx += dx
                            totalDy += dy

                            if (!isDraggingDown) {
                                if (abs(totalDy) > touchSlop &&
                                    abs(totalDy) > abs(totalDx) * 1.5f &&
                                    totalDy > 0f
                                ) {
                                    isDraggingDown = true
                                    rawDragY = settleDragY.value
                                    isActivelyDragging = true
                                    coroutineScope.launch { settleDragY.stop() }
                                    change.consume()
                                }
                            } else if (dy != 0f) {
                                change.consume()
                                rawDragY = (rawDragY + dy).coerceAtLeast(0f)
                            }

                            if (!change.pressed) break
                        }
                    } finally {
                        if (isDraggingDown) {
                            val releaseY = rawDragY
                            coroutineScope.launch {
                                settleDragY.snapTo(releaseY)
                                isActivelyDragging = false
                                if (releaseY > dismissThresholdPx) {
                                    triggerDismiss()
                                } else {
                                    settleDragY.animateTo(
                                        targetValue = 0f,
                                        animationSpec = spring(
                                            dampingRatio = Spring.DampingRatioNoBouncy,
                                            stiffness = Spring.StiffnessMedium
                                        )
                                    )
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

        // Interactive transformed viewer container
        Box(
            modifier = Modifier
                .fillMaxSize()
                .graphicsLayer {
                    translationY = currentDragY
                    scaleX = viewerScale
                    scaleY = viewerScale
                }
        ) {
            val isPagerScrollEnabled = isViewerVisible && !isZoomed

            if (items.isNotEmpty()) {
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
                                isActivePage = isCurrentPage,
                                isSettledPage = isSettledPage,
                                isViewerVisible = isViewerVisible,
                                allowSharedBoundsAnimation = allowSharedBoundsAnimation,
                                sharedTransitionScope = sharedTransitionScope,
                                memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                                onTap = {
                                    if (currentIsViewerVisible) viewModel.toggleChrome()
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

            val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
            val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
            val showChrome = isViewerVisible && uiState.isChromeVisible

            AnimatedVisibility(
                visible = showChrome,
                enter = fadeIn() + slideInVertically { -it },
                exit = fadeOut() + slideOutVertically { -it },
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = statusBarPadding + 6.dp, start = 8.dp, end = 8.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    ViewerCircleButton(
                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Back",
                        onClick = { triggerDismiss() }
                    )

                    Spacer(modifier = Modifier.width(8.dp))

                    // Updated directly without animateContentSize() to prevent mid-swipe relayouts
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
            }

            // Bottom Floating Pill: Share, Add to, Bin (placeholders until 2B / Phases 4-5)
            AnimatedVisibility(
                visible = showChrome,
                enter = fadeIn() + slideInVertically { it },
                exit = fadeOut() + slideOutVertically { it },
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = navBarPadding + 14.dp)
            ) {
                Surface(
                    shape = CircleShape,
                    color = ViewerPillColor,
                    border = ViewerPillBorder
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

private const val BACKDROP_FADE_MS = 180

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