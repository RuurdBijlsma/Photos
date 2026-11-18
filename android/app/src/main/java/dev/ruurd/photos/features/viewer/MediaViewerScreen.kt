package dev.ruurd.photos.features.viewer

import android.app.Activity
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
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
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.material.icons.filled.Info
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
import androidx.core.view.WindowInsetsControllerCompat
import coil3.ImageLoader
import coil3.request.ImageRequest
import dev.ruurd.photos.core.image.MediaStoreThumbnail
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.features.viewer.components.ZoomablePhotoPage
import dev.ruurd.photos.ui.theme.MontserratFontFamily
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlin.math.abs

@Composable
fun MediaViewerScreen(
    viewModel: MediaViewerViewModel,
    baseUrl: String,
    imageLoader: ImageLoader,
    memoryCacheKeyRegistry: MemoryCacheKeyRegistry,
    onDismiss: () -> Unit,
    onPreSyncScroll: (mediaId: String) -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val items = uiState.items
    val isDarkTheme = isSystemInDarkTheme()
    val context = LocalContext.current
    val viewConfig = LocalViewConfiguration.current
    val coroutineScope = rememberCoroutineScope()

    val initialIndex = remember { uiState.initialIndex.coerceIn(0, (items.size - 1).coerceAtLeast(0)) }
    val pagerState = rememberPagerState(
        initialPage = initialIndex,
        pageCount = { items.size }
    )

    // Single-pointer drag dismiss animation & offset
    val dragOffsetY = remember { Animatable(0f) }
    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f

    // Synchronize page changes with ViewModel activeMediaId and reset zoom fraction
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.currentPage }.collect { page ->
            activePageZoomFraction = 0f
            if (page in items.indices) {
                viewModel.onPageChanged(items[page].id)
            }
        }
    }

    // Silent pre-sync of timeline scroll position ~250ms after settledPage pauses
    LaunchedEffect(pagerState, items) {
        snapshotFlow { pagerState.settledPage }.collectLatest { settledIndex ->
            delay(250L)
            if (settledIndex in items.indices) {
                onPreSyncScroll(items[settledIndex].id)
            }
        }
    }

    // Neighbor prefetch for currentPage Â± 1 previews after ~120-150ms settle delay
    LaunchedEffect(pagerState, items, baseUrl) {
        snapshotFlow { pagerState.settledPage }.collectLatest { settledIndex ->
            delay(130L)
            val neighbors = listOf(settledIndex - 1, settledIndex + 1).filter { it in items.indices }
            for (idx in neighbors) {
                val neighbor = items[idx]
                val localUri = neighbor.localContentUri?.let(Uri::parse)
                val fallbackUrl = when {
                    !neighbor.isVideo && !neighbor.cloudId.isNullOrEmpty() -> {
                        if (neighbor.hasThumbnails) {
                            "$baseUrl/thumbnails/${neighbor.cloudId}/1440p.avif"
                        } else {
                            "$baseUrl/api/photos/${neighbor.cloudId}/thumbnail?size=1440"
                        }
                    }
                    !neighbor.cloudId.isNullOrEmpty() -> {
                        if (neighbor.hasThumbnails) {
                            "$baseUrl/thumbnails/${neighbor.cloudId}/720p.avif"
                        } else {
                            "$baseUrl/api/photos/${neighbor.cloudId}/thumbnail?size=720"
                        }
                    }
                    else -> null
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
                    val targetHeight = if (neighbor.isVideo) 720 else 1440
                    val req = ImageRequest.Builder(context)
                        .data(previewModel)
                        .size(targetHeight, targetHeight)
                        .build()
                    imageLoader.enqueue(req)
                }
            }
        }
    }

    // System bar appearance management:
    // Keep navigation bar transparent edge-to-edge so the back gesture is never intercepted by Android's
    // immersive swipe gesture. Hide only the status bar when chrome is hidden.
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

    // Toggle status bar appearance with chrome visibility
    LaunchedEffect(uiState.isChromeVisible) {
        if (uiState.isChromeVisible) {
            insetsController?.show(WindowInsetsCompat.Type.statusBars())
        } else {
            insetsController?.hide(WindowInsetsCompat.Type.statusBars())
        }
    }

    BackHandler(enabled = true) {
        onDismiss()
    }

    // Backdrop alpha fades proportionally during interactive swipe-down
    val backdropAlpha = (1f - (dragOffsetY.value / 600f)).coerceIn(0f, 1f)
    val viewerScale = (1f - (dragOffsetY.value / 1800f)).coerceIn(0.85f, 1f)

    val currentIsZoomed by rememberUpdatedState(isZoomed)
    val dismissThresholdPx = with(LocalDensity.current) { 120.dp.toPx() }

    // Shared by every way a drag can end, so a cancelled or aborted gesture can never leave the
    // viewer stuck half-dragged with a transparent backdrop.
    suspend fun settleDrag() {
        if (dragOffsetY.value > dismissThresholdPx) {
            // Restore light system bars immediately on dismiss commit in light mode
            if (!isDarkTheme) {
                insetsController?.isAppearanceLightStatusBars = true
                insetsController?.isAppearanceLightNavigationBars = true
            }
            onDismiss()
        } else {
            dragOffsetY.animateTo(
                targetValue = 0f,
                animationSpec = spring(
                    dampingRatio = Spring.DampingRatioNoBouncy,
                    stiffness = Spring.StiffnessMedium
                )
            )
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = backdropAlpha))
            .graphicsLayer {
                translationY = dragOffsetY.value
                scaleX = viewerScale
                scaleY = viewerScale
            }
            .pointerInput(Unit) {
                val touchSlop = viewConfig.touchSlop

                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    // Read the zoom at gesture start instead of keying pointerInput on it: restarting
                    // the handler mid-drag would drop the settle below.
                    if (currentIsZoomed) return@awaitEachGesture
                    val pointerId = down.id
                    var totalDx = 0f
                    var totalDy = 0f
                    var isDraggingDown = false

                    try {
                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull { it.id == pointerId } ?: break

                            // A change the pager or a pinch consumed is not ours, and a second
                            // pointer means the user is zooming.
                            if (change.isConsumed || event.changes.size > 1) break

                            val dx = change.position.x - change.previousPosition.x
                            val dy = change.position.y - change.previousPosition.y
                            totalDx += dx
                            totalDy += dy

                            if (!isDraggingDown) {
                                if (abs(totalDy) > touchSlop && abs(totalDy) > abs(totalDx) * 1.5f && totalDy > 0f) {
                                    isDraggingDown = true
                                    change.consume()
                                }
                            } else if (dy != 0f) {
                                change.consume()
                                coroutineScope.launch {
                                    dragOffsetY.snapTo((dragOffsetY.value + dy).coerceAtLeast(0f))
                                }
                            }

                            if (!change.pressed) break
                        }
                    } finally {
                        if (isDraggingDown) {
                            coroutineScope.launch { settleDrag() }
                        }
                    }
                }
            }
    ) {
        val isPagerScrollEnabled = !isZoomed

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
                        ZoomablePhotoPage(
                            item = item,
                            baseUrl = baseUrl,
                            isActivePage = pagerState.currentPage == page,
                            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                            onTap = { viewModel.toggleChrome() },
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

        // Floating chrome. Top and bottom animate separately: sharing one slide would also
        // slide the bottom pill off the top edge when the chrome hides.
        val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
        val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

        AnimatedVisibility(
            visible = uiState.isChromeVisible,
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
                    onClick = onDismiss
                )

                Spacer(modifier = Modifier.width(8.dp))

                // Title / counter pill: updated directly without animateContentSize() to prevent mid-swipe springs
                Surface(
                    shape = CircleShape,
                    color = ViewerPillColor,
                    border = ViewerPillBorder
                ) {
                    val counterText = if (items.isNotEmpty()) {
                        "${pagerState.currentPage + 1} of ${items.size}"
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

                // Top-right overflow â‹® menu pill
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
                        DropdownMenuItem(
                            text = { Text("Reprocess") },
                            onClick = { isMenuExpanded = false }
                        )
                    }
                }
            }
        }

        // Bottom Floating Pill: Share, Add to, Bin (placeholders until 2B / Phases 4-5)
        AnimatedVisibility(
            visible = uiState.isChromeVisible,
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
