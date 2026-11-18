package dev.ruurd.photos.features.viewer

import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.testing.FakeMediaRepository
import kotlinx.coroutines.CoroutineDispatcher
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

@OptIn(ExperimentalCoroutinesApi::class)
class MediaViewerViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private val testScope = TestScope(testDispatcher)

    private val dispatchers = object : AppDispatchers {
        override val default: CoroutineDispatcher = testDispatcher
        override val io: CoroutineDispatcher = testDispatcher
        override val main: CoroutineDispatcher = testDispatcher
    }

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
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611)
            )
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            dispatchers = dispatchers
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p2", state.activeMediaId)
        assertEquals(3, state.items.size)
        assertEquals(1, state.initialIndex)

        // Page change
        viewModel.onPageChanged("p3")
        assertEquals("p3", viewModel.uiState.value.activeMediaId)

        // Chrome toggle
        assertTrue(viewModel.uiState.value.isChromeVisible)
        viewModel.toggleChrome()
        assertFalse(viewModel.uiState.value.isChromeVisible)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `falls back to repository item if store is loading`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsert(MediaEntity(id = "p1", sortTimestamp = 1000L, yearMonth = 202611))

        // Create store but don't run scope yet (stays in Loading initially)
        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers
        )

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p1",
            timelineProjectionStore = store,
            mediaRepository = repo,
            dispatchers = dispatchers
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p1", state.activeMediaId)
        assertEquals(1, state.items.size)
        assertEquals("p1", state.items[0].id)

        viewModel.viewModelScope.cancel()
        store.stop()
    }
}
