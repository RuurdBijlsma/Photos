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
import androidx.compose.foundation.background
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
import androidx.compose.material3.IconButton
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
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
    val coroutineScope = rememberCoroutineScope()
    val viewConfig = LocalViewConfiguration.current

    val initialIndex = remember { uiState.initialIndex.coerceIn(0, (items.size - 1).coerceAtLeast(0)) }
    val pagerState = rememberPagerState(
        initialPage = initialIndex,
        pageCount = { items.size }
    )

    // Synchronize page changes with ViewModel activeMediaId
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.currentPage }.collect { page ->
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

    // Neighbor prefetch for currentPage ± 1 previews after ~120-150ms settle delay
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

    // Single-pointer drag dismiss animation & offset
    val dragOffsetY = remember { Animatable(0f) }
    var activePageZoomFraction by remember { mutableFloatStateOf(0f) }
    val isZoomed = activePageZoomFraction > 0.01f

    // System bar appearance management
    val activity = context as? Activity
    val window = activity?.window
    val insetsController = remember(window) {
        window?.let { WindowCompat.getInsetsController(it, it.decorView) }
    }

    DisposableEffect(window, isDarkTheme) {
        insetsController?.apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
        onDispose {
            insetsController?.apply {
                isAppearanceLightStatusBars = !isDarkTheme
                isAppearanceLightNavigationBars = !isDarkTheme
                show(WindowInsetsCompat.Type.systemBars())
            }
        }
    }

    // Toggle system bars with chrome visibility
    LaunchedEffect(uiState.isChromeVisible) {
        if (uiState.isChromeVisible) {
            insetsController?.show(WindowInsetsCompat.Type.systemBars())
        } else {
            insetsController?.hide(WindowInsetsCompat.Type.systemBars())
        }
    }

    BackHandler(enabled = true) {
        onDismiss()
    }

    // Backdrop alpha fades proportionally during interactive swipe-down
    val backdropAlpha = (1f - (dragOffsetY.value / 600f)).coerceIn(0f, 1f)
    val viewerScale = (1f - (dragOffsetY.value / 1800f)).coerceIn(0.85f, 1f)

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = backdropAlpha))
            .graphicsLayer {
                translationY = dragOffsetY.value
                scaleX = viewerScale
                scaleY = viewerScale
            }
            .pointerInput(isZoomed) {
                if (isZoomed) return@pointerInput
                val touchSlop = viewConfig.touchSlop
                val dismissThresholdPx = 120.dp.toPx()

                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    val pointerId = down.id
                    var totalDx = 0f
                    var totalDy = 0f
                    var isDraggingDown = false

                    while (true) {
                        val event = awaitPointerEvent()
                        val change = event.changes.firstOrNull { it.id == pointerId } ?: break
                        if (change.isConsumed) break

                        if (event.changes.size > 1) {
                            // Multi-touch (pinch) -> abort swipe down so zoom is not stolen
                            break
                        }

                        val dx = change.position.x - change.previousPosition.x
                        val dy = change.position.y - change.previousPosition.y
                        totalDx += dx
                        totalDy += dy

                        if (!isDraggingDown) {
                            if (abs(totalDy) > touchSlop && abs(totalDy) > abs(totalDx) * 1.5f && totalDy > 0f) {
                                isDraggingDown = true
                                change.consume()
                            }
                        } else {
                            if (dy != 0f) {
                                change.consume()
                                coroutineScope.launch {
                                    dragOffsetY.snapTo((dragOffsetY.value + dy).coerceAtLeast(0f))
                                }
                            }
                        }

                        if (!change.pressed) {
                            if (isDraggingDown) {
                                if (dragOffsetY.value > dismissThresholdPx) {
                                    // Restore light system bars immediately on dismiss commit in light mode
                                    if (!isDarkTheme) {
                                        insetsController?.isAppearanceLightStatusBars = true
                                        insetsController?.isAppearanceLightNavigationBars = true
                                    }
                                    onDismiss()
                                } else {
                                    coroutineScope.launch {
                                        dragOffsetY.animateTo(
                                            targetValue = 0f,
                                            animationSpec = spring(
                                                dampingRatio = Spring.DampingRatioNoBouncy,
                                                stiffness = Spring.StiffnessMedium
                                            )
                                        )
                                    }
                                }
                            }
                            break
                        }
                    }
                }
            }
    ) {
        val isPagerScrollEnabled = !isZoomed && dragOffsetY.value == 0f

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
                            isSettledPage = pagerState.settledPage == page,
                            memoryCacheKeyRegistry = memoryCacheKeyRegistry,
                            onTap = { viewModel.toggleChrome() },
                            onZoomableStateReady = { zoomState ->
                                if (pagerState.currentPage == page) {
                                    activePageZoomFraction = zoomState.zoomFraction ?: 0f
                                }
                            },
                            modifier = Modifier.fillMaxSize()
                        )
                    }
                }
            }
        }

        // Floating Chrome (Top Bar + Bottom Pill)
        AnimatedVisibility(
            visible = uiState.isChromeVisible,
            enter = fadeIn() + slideInVertically { -it / 2 },
            exit = fadeOut() + slideOutVertically { -it / 2 },
            modifier = Modifier.fillMaxSize()
        ) {
            Box(modifier = Modifier.fillMaxSize()) {
                val statusBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
                val navBarPadding = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()

                // Top Bar: Back pill + Counter/Title pill + Overflow Menu pill
                Row(
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = statusBarPadding + 8.dp, start = 16.dp, end = 16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Back button pill
                    Surface(
                        shape = CircleShape,
                        color = Color.Black.copy(alpha = 0.55f),
                        modifier = Modifier.size(44.dp)
                    ) {
                        IconButton(onClick = onDismiss) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back",
                                tint = Color.White
                            )
                        }
                    }

                    Spacer(modifier = Modifier.width(12.dp))

                    // Title / counter pill: updated directly without animateContentSize() to prevent mid-swipe springs
                    Surface(
                        shape = CircleShape,
                        color = Color.Black.copy(alpha = 0.55f),
                        modifier = Modifier.weight(1f, fill = false)
                    ) {
                        val counterText = if (items.isNotEmpty()) {
                            "${pagerState.currentPage + 1} of ${items.size}"
                        } else {
                            ""
                        }
                        Text(
                            text = counterText,
                            style = MaterialTheme.typography.titleMedium.copy(
                                fontFamily = MontserratFontFamily,
                                fontWeight = FontWeight.SemiBold
                            ),
                            color = Color.White,
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp)
                        )
                    }

                    Spacer(modifier = Modifier.weight(1f))

                    // Top-right overflow ⋮ menu pill
                    var isMenuExpanded by remember { mutableStateOf(false) }
                    Box {
                        Surface(
                            shape = CircleShape,
                            color = Color.Black.copy(alpha = 0.55f),
                            modifier = Modifier.size(44.dp)
                        ) {
                            IconButton(onClick = { isMenuExpanded = true }) {
                                Icon(
                                    imageVector = Icons.Default.MoreVert,
                                    contentDescription = "Options",
                                    tint = Color.White
                                )
                            }
                        }

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

                // Bottom Floating Pill: Share, Add to, Bin (placeholder / disabled in Phase 2A)
                Surface(
                    shape = CircleShape,
                    color = Color.Black.copy(alpha = 0.55f),
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = navBarPadding + 16.dp)
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // Share button
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.alpha(0.5f)
                        ) {
                            IconButton(onClick = {}, enabled = false) {
                                Icon(
                                    imageVector = Icons.Default.Share,
                                    contentDescription = "Share",
                                    tint = Color.White
                                )
                            }
                            Text(
                                text = "Share",
                                style = MaterialTheme.typography.labelMedium,
                                color = Color.White
                            )
                        }

                        // Add to album button
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.alpha(0.5f)
                        ) {
                            IconButton(onClick = {}, enabled = false) {
                                Icon(
                                    imageVector = Icons.Default.Add,
                                    contentDescription = "Add to",
                                    tint = Color.White
                                )
                            }
                            Text(
                                text = "Add to",
                                style = MaterialTheme.typography.labelMedium,
                                color = Color.White
                            )
                        }

                        // Bin button
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.alpha(0.5f)
                        ) {
                            IconButton(onClick = {}, enabled = false) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = "Bin",
                                    tint = Color.White
                                )
                            }
                            Text(
                                text = "Bin",
                                style = MaterialTheme.typography.labelMedium,
                                color = Color.White
                            )
                        }
                    }
                }
            }
        }
    }
}
