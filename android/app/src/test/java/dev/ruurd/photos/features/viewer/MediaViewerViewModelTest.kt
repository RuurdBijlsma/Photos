package dev.ruurd.photos.features.viewer

import android.content.Intent
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.MediaFileSharer
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoSource
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import dev.ruurd.photos.testing.FakeMediaRepository
import dev.ruurd.photos.testing.TestAppDispatchers
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
class MediaViewerViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private val testScope = TestScope(testDispatcher)

    private val dispatchers = TestAppDispatchers(testDispatcher)

    private val detailsRepository = FakeMediaDetailsRepository(
        mapOf(
            "c1" to MediaDetails(cloudId = "c1", fileName = "one.jpg"),
            "c2" to MediaDetails(cloudId = "c2", fileName = "two.jpg"),
            "c3" to MediaDetails(cloudId = "c3", fileName = "three.jpg"),
        ),
    )

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `initializes with warm store and anchors initial page`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p2", state.activeMediaId)
        assertEquals(3, state.items.size)
        assertEquals(1, state.initialIndex)

        // Page change updates activeMediaId and initialIndex
        viewModel.onPageChanged("p3")
        assertEquals("p3", viewModel.uiState.value.activeMediaId)
        assertEquals(2, viewModel.uiState.value.initialIndex)

        // Chrome toggle
        assertTrue(viewModel.uiState.value.isChromeVisible)
        viewModel.toggleChrome()
        assertFalse(viewModel.uiState.value.isChromeVisible)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `re-anchors initialIndex when store transitions from Loading fallback to Loaded`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        // Store has not collected yet -> starts in Loading
        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p2", state.activeMediaId)
        assertEquals(3, state.items.size)
        assertEquals(1, state.initialIndex)
        assertFalse(state.isActiveItemMissing)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `keeps activeMediaId anchored when newer items are prepended`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()
        assertEquals(1, viewModel.uiState.value.initialIndex)

        // Prepend a newer item -> p2 shifts from index 1 to index 2
        repo.upsert(MediaEntity(id = "p0", sortTimestamp = 4000L, yearMonth = 202611))
        runCurrent()

        val updated = viewModel.uiState.value
        assertEquals("p2", updated.activeMediaId)
        assertEquals(4, updated.items.size)
        assertEquals(2, updated.initialIndex)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `clamps to neighbor when active item is deleted and flags missing when library empties`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        repo.deleteById("p2")
        runCurrent()

        val clamped = viewModel.uiState.value
        assertEquals("p1", clamped.activeMediaId)
        assertEquals(0, clamped.initialIndex)
        assertFalse(clamped.isActiveItemMissing)

        repo.deleteById("p1")
        runCurrent()

        assertTrue(viewModel.uiState.value.isActiveItemMissing)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `settling loads item details for the page and prefetches its neighbors`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", cloudId = "c1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", cloudId = "c2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", cloudId = "c3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        viewModel.onSettledPage("p2")
        runCurrent()

        assertEquals("c2", viewModel.uiState.value.activeDetails?.cloudId)
        assertEquals("c1", viewModel.uiState.value.detailsById["p1"]?.cloudId)
        assertEquals("c3", viewModel.uiState.value.detailsById["p3"]?.cloudId)
        assertTrue(detailsRepository.requested.containsAll(listOf("c2", "c1", "c3")))

        // Swiping to a prefetched neighbor keeps its details available immediately.
        viewModel.onPageChanged("p3")
        assertEquals("c3", viewModel.uiState.value.activeDetails?.cloudId)

        viewModel.openInfoSheet()
        assertTrue(viewModel.uiState.value.isInfoSheetOpen)
        viewModel.closeInfoSheet()
        assertFalse(viewModel.uiState.value.isInfoSheetOpen)

        viewModel.viewModelScope.cancel()
        store.stop()
    }
}

private class FakeMediaDetailsRepository(
    private val items: Map<String, MediaDetails> = emptyMap(),
) : MediaDetailsRepository {
    val requested = mutableListOf<String>()

    override fun peek(cloudId: String): MediaDetails? = items[cloudId]

    override suspend fun get(cloudId: String): MediaDetails? {
        requested += cloudId
        return items[cloudId]
    }

    override fun evict(cloudIds: Collection<String>) = Unit

    override fun clear() = Unit
}

private class FakeFullResFileStore : FullResFileStore {
    override fun peek(cloudId: String): File? = null

    override suspend fun get(cloudId: String, fileName: String): File? = null

    override suspend fun evict(cloudId: String) = Unit

    override suspend fun clear() = Unit
}

private class FakeLocalMediaInfoSource : LocalMediaInfoSource {
    override suspend fun read(contentUri: String, isVideo: Boolean): LocalMediaInfo? = null
}

private class FakeMediaFileSharer : MediaFileSharer {
    override suspend fun shareIntent(item: TimelineMediaProjection, details: MediaDetails?): Intent? = null

    override suspend fun copyToClipboard(item: TimelineMediaProjection, details: MediaDetails?): Boolean = false
}