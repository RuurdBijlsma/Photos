package dev.ruurd.photos.core.state

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class BootstrapStateTest {

    private val testDispatcher = StandardTestDispatcher()

    @Test
    fun `bootstraps until sync and scan both settle`() = runTest(testDispatcher) {
        val state = BootstrapState(needsBootstrap = true)
        val seen = mutableListOf<Boolean>()
        val job = launch { state.isBootstrapping.collect { seen.add(it) } }
        runCurrent()

        assertEquals(listOf(true), seen)

        state.onSyncSettled()
        runCurrent()
        assertEquals(true, seen.last())

        state.onScanSettled()
        runCurrent()
        assertEquals(false, seen.last())

        state.reset()
        runCurrent()
        assertEquals(true, seen.last())

        job.cancel()
    }

    @Test
    fun `returning session does not bootstrap`() = runTest(testDispatcher) {
        val state = BootstrapState(needsBootstrap = false)
        val seen = mutableListOf<Boolean>()
        val job = launch { state.isBootstrapping.collect { seen.add(it) } }
        runCurrent()

        assertEquals(listOf(false), seen)

        job.cancel()
    }
}
