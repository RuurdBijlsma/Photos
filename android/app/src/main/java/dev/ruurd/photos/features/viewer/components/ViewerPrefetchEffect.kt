package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.platform.LocalContext
import coil3.SingletonImageLoader
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.MediaImageRequests
import kotlinx.collections.immutable.ImmutableList
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull

/** Warms the current page ± 1 previews ~130 ms after the pager settles. */
@Composable
fun ViewerPrefetchEffect(
    pagerState: PagerState,
    items: ImmutableList<TimelineMediaProjection>,
    baseUrl: String,
) {
    val currentItems by rememberUpdatedState(items)
    val context = LocalContext.current
    val imageLoader = SingletonImageLoader.get(context)

    LaunchedEffect(pagerState, baseUrl, imageLoader) {
        snapshotFlow {
            if (currentItems.isEmpty()) null else pagerState.settledPage to currentItems
        }
            .filterNotNull()
            .collectLatest { (settledIndex, latestItems) ->
                delay(PREFETCH_SETTLE_DELAY_MS)
                val neighbors = listOf(settledIndex - 1, settledIndex + 1)
                    .filter { it in latestItems.indices }
                for (idx in neighbors) {
                    val neighbor = latestItems[idx]
                    val request = MediaImageRequests.previewRequest(
                        context = context,
                        localContentUri = neighbor.localContentUri,
                        cloudId = neighbor.cloudId,
                        isVideo = neighbor.isVideo,
                        hasThumbnails = neighbor.hasThumbnails,
                        baseUrl = baseUrl,
                    )
                    if (request != null) {
                        imageLoader.enqueue(request)
                    }
                }
            }
    }
}

/** Silently pre-scrolls the timeline under the viewer ~250 ms after the pager settles. */
@Composable
fun ViewerPreSyncScrollEffect(
    pagerState: PagerState,
    items: ImmutableList<TimelineMediaProjection>,
    isClosing: () -> Boolean,
    onPreSyncScroll: (String) -> Unit,
) {
    val currentItems by rememberUpdatedState(items)
    val currentIsClosing by rememberUpdatedState(isClosing)
    val currentOnPreSyncScroll by rememberUpdatedState(onPreSyncScroll)

    LaunchedEffect(pagerState) {
        snapshotFlow {
            if (currentItems.isEmpty()) null else pagerState.settledPage
        }
            .filterNotNull()
            .collectLatest { settledIndex ->
                delay(PRESYNC_SCROLL_DELAY_MS)
                val latestItems = currentItems
                if (settledIndex in latestItems.indices && !currentIsClosing()) {
                    currentOnPreSyncScroll(latestItems[settledIndex].id)
                }
            }
    }
}

private const val PREFETCH_SETTLE_DELAY_MS = 130L
private const val PRESYNC_SCROLL_DELAY_MS = 250L
