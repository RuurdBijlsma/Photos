package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.testing.FakeMediaRepository
import dev.ruurd.photos.testing.TestAppDispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class TimelineProjectionStoreTest {

    private val testDispatcher = StandardTestDispatcher()
    private val testScope = TestScope(testDispatcher)

    private val dispatchers = TestAppDispatchers(testDispatcher)

    @Test
    fun `emits loaded state from repository and updates on changes`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsert(MediaEntity(id = "photo_1", sortTimestamp = 1000L, yearMonth = 202611))

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers
        )

        runCurrent()

        val state = store.state.value
        assertTrue(state is TimelineProjectionState.Loaded)
        val loaded = state as TimelineProjectionState.Loaded
        assertEquals(1, loaded.items.size)
        assertEquals("photo_1", loaded.items[0].id)
        assertNotNull(store.getById("photo_1"))

        // Add another photo
        repo.upsert(MediaEntity(id = "photo_2", sortTimestamp = 2000L, yearMonth = 202611))
        runCurrent()

        val updatedState = store.state.value as TimelineProjectionState.Loaded
        assertEquals(2, updatedState.items.size)

        // Clear resets and reconnects
        store.clear()
        runCurrent()
        val afterClear = store.state.value
        assertTrue(afterClear is TimelineProjectionState.Loaded)

        store.stop()
    }
}
