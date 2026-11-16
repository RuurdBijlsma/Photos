package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.testing.FakeMediaRepository
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ObserveTimelineUseCaseTest {

    private val testDispatcher = StandardTestDispatcher()

    private val dispatchers = object : AppDispatchers {
        override val default: CoroutineDispatcher = testDispatcher
        override val io: CoroutineDispatcher = testDispatcher
        override val main: CoroutineDispatcher = testDispatcher
    }

    private fun entity(id: String, yearMonth: Int) = MediaEntity(
        id = id,
        yearMonth = yearMonth,
        sortTimestamp = yearMonth.toLong()
    )

    @Test
    fun `emits stage 1 immediately then coalesces stage 2 updates`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                entity("m1", 202611),
                entity("m2", 202610),
                entity("m3", 202609)
            )
        )

        val emissions = mutableListOf<List<TimelineMediaProjection>>()
        val job = launch {
            ObserveTimelineUseCase(repo, dispatchers)().collect { emissions.add(it) }
        }

        // Stage 1 arrives immediately with only the newest two months.
        runCurrent()
        assertEquals(1, emissions.size)
        assertEquals(listOf("m1", "m2"), emissions[0].map { it.id })

        // Rapid updates within the 150ms debounce window coalesce into one Stage 2 emission.
        advanceTimeBy(50)
        repo.upsert(entity("m4", 202608))
        advanceTimeBy(50)
        repo.upsert(entity("m5", 202607))

        advanceTimeBy(149)
        runCurrent()
        assertEquals(1, emissions.size)

        advanceTimeBy(1)
        runCurrent()
        assertEquals(2, emissions.size)
        assertEquals(listOf("m1", "m2", "m3", "m4", "m5"), emissions[1].map { it.id })

        job.cancel()
    }
}