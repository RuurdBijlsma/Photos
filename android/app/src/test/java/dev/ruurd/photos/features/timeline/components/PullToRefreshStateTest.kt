package dev.ruurd.photos.features.main.components

import androidx.compose.runtime.MonotonicFrameClock
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PullToRefreshStateTest {

    private class TestFrameClock : MonotonicFrameClock {
        private var frameTimeNanos = 0L

        override suspend fun <R> withFrameNanos(onFrame: (frameTimeNanos: Long) -> R): R {
            frameTimeNanos += 16_000_000L
            return onFrame(frameTimeNanos)
        }
    }

    private fun createState(
        scope: TestScope,
        isOnline: Boolean = true,
        isRefreshing: Boolean = false,
        onRefresh: () -> Unit = {},
        onOfflineRefresh: () -> Unit = {},
        haptics: MutableList<HapticFeedbackType> = mutableListOf(),
    ) = PullToRefreshState(
        scope = scope,
        thresholdRawPx = 300f,
        maxVisualOffsetPx = 200f,
        performHaptic = { haptics.add(it) },
        isOnline = isOnline,
        isRefreshing = isRefreshing,
        onRefresh = onRefresh,
        onOfflineRefresh = onOfflineRefresh,
    )

    @Test
    fun `arms at threshold and fires threshold haptic once`() = runTest(TestFrameClock()) {
        val haptics = mutableListOf<HapticFeedbackType>()
        val state = createState(this, haptics = haptics)

        state.onPullDelta(150f)
        assertTrue(state.isPulling)
        assertFalse(state.isArmed)
        assertEquals(0.5f, state.pullProgress, 0.001f)
        assertTrue(haptics.isEmpty())

        state.onPullDelta(150f)
        assertTrue(state.isArmed)
        assertEquals(1.0f, state.pullProgress, 0.001f)
        assertEquals(listOf(HapticFeedbackType.GestureThresholdActivate), haptics)

        // Further pulling does not repeat the threshold haptic
        state.onPullDelta(60f)
        assertEquals(1, haptics.size)
    }

    @Test
    fun `upward drag reverses pull before returning zero consumption`() = runTest(TestFrameClock()) {
        val state = createState(this)

        assertEquals(120f, state.onPullDelta(120f), 0.001f)
        assertEquals(-50f, state.onPullDelta(-50f), 0.001f)
        assertEquals(70f / 300f, state.pullProgress, 0.001f)

        // Pulling back past zero only consumes the remaining pull distance
        assertEquals(-70f, state.onPullDelta(-110f), 0.001f)
        assertFalse(state.isPulled)
        assertEquals(0f, state.contentOffsetPx, 0.001f)
    }

    @Test
    fun `releasing past threshold when online triggers refresh and settles to zero`() = runTest(TestFrameClock()) {
        val haptics = mutableListOf<HapticFeedbackType>()
        var refreshCalls = 0
        var offlineCalls = 0
        val state = createState(
            scope = this,
            isOnline = true,
            onRefresh = { refreshCalls++ },
            onOfflineRefresh = { offlineCalls++ },
            haptics = haptics,
        )

        state.onPullDelta(320f)
        state.onRelease()

        assertEquals(1, refreshCalls)
        assertEquals(0, offlineCalls)
        assertEquals(
            listOf(HapticFeedbackType.GestureThresholdActivate, HapticFeedbackType.Confirm),
            haptics,
        )

        advanceUntilIdle()
        assertFalse(state.isPulled)
        assertEquals(0f, state.contentOffsetPx, 0.001f)
    }

    @Test
    fun `releasing past threshold when offline triggers offline feedback and local refresh only`() = runTest(TestFrameClock()) {
        val haptics = mutableListOf<HapticFeedbackType>()
        var refreshCalls = 0
        var offlineCalls = 0
        val state = createState(
            scope = this,
            isOnline = false,
            onRefresh = { refreshCalls++ },
            onOfflineRefresh = { offlineCalls++ },
            haptics = haptics,
        )

        state.onPullDelta(320f)
        state.onRelease()

        assertEquals(0, refreshCalls)
        assertEquals(1, offlineCalls)
        assertEquals(1, state.offlineFeedbackCount)
        assertEquals(
            listOf(HapticFeedbackType.GestureThresholdActivate, HapticFeedbackType.Reject),
            haptics,
        )
        advanceUntilIdle()
    }

    @Test
    fun `releasing before threshold does not trigger refresh`() = runTest(TestFrameClock()) {
        var refreshCalls = 0
        val state = createState(this, onRefresh = { refreshCalls++ })

        state.onPullDelta(200f)
        state.onRelease()
        advanceUntilIdle()

        assertEquals(0, refreshCalls)
        assertFalse(state.isPulled)
    }

    @Test
    fun `does not re-trigger while already refreshing`() = runTest(TestFrameClock()) {
        var refreshCalls = 0
        val haptics = mutableListOf<HapticFeedbackType>()
        val state = createState(
            scope = this,
            isRefreshing = true,
            onRefresh = { refreshCalls++ },
            haptics = haptics,
        )

        state.onPullDelta(350f)
        state.onRelease()
        advanceUntilIdle()

        assertEquals(0, refreshCalls)
        assertTrue(haptics.isEmpty())
    }
}
