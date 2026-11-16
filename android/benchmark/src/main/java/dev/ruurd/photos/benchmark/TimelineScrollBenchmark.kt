package dev.ruurd.photos.benchmark

import android.content.ComponentName
import android.content.Intent
import android.os.SystemClock
import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.MacrobenchmarkScope
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.StartupTimingMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiObject2
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TimelineScrollBenchmark {

    @get:Rule
    val benchmarkRule = MacrobenchmarkRule()

    @Test
    fun coldStartupAndScroll() = benchmarkRule.measureRepeated(
        packageName = TARGET_PACKAGE,
        metrics = listOf(StartupTimingMetric(), FrameTimingMetric()),
        compilationMode = CompilationMode.None(),
        startupMode = StartupMode.COLD,
        iterations = 5,
        setupBlock = { pressHome() }
    ) {
        startActivityAndWait(mainActivityIntent())
        val list = awaitTimelineList()

        // Scroll immediately after cold launch while startup scan/sync/layout work is active.
        continuousScroll(list, downSwipes = 4, upSwipes = 0, steps = 14)
    }

    @Test
    fun scrollTimelineContinuous() = benchmarkRule.measureRepeated(
        packageName = TARGET_PACKAGE,
        metrics = listOf(FrameTimingMetric()),
        compilationMode = CompilationMode.Partial(),
        startupMode = StartupMode.WARM,
        iterations = 5,
        setupBlock = {
            pressHome()
            startActivityAndWait(mainActivityIntent())
            awaitTimelineList()
            device.waitForIdle()
        }
    ) {
        val list = awaitTimelineList()
        continuousScroll(list, downSwipes = 6, upSwipes = 4, steps = 14)
        device.waitForIdle()
    }

    @Test
    fun scrubTimeline() = benchmarkRule.measureRepeated(
        packageName = TARGET_PACKAGE,
        metrics = listOf(FrameTimingMetric()),
        compilationMode = CompilationMode.Partial(),
        startupMode = StartupMode.WARM,
        iterations = 5,
        setupBlock = {
            pressHome()
            startActivityAndWait(mainActivityIntent())
            awaitTimelineList()
            device.waitForIdle()
        }
    ) {
        val scrubber = device.wait(Until.findObject(By.res("timeline_scrubber")), 10_000)
            ?: error("timeline_scrubber not found (library may be shorter than one screen)")
        val bounds = scrubber.visibleBounds
        val x = bounds.centerX()
        val topY = bounds.top + (bounds.height() * 0.15f).toInt()
        val bottomY = bounds.top + (bounds.height() * 0.85f).toInt()

        device.swipe(x, topY, x, bottomY, 45)
        SystemClock.sleep(100)
        device.swipe(x, bottomY, x, topY, 45)
        device.waitForIdle()
    }

    private fun MacrobenchmarkScope.awaitTimelineList(): UiObject2 =
        device.wait(Until.findObject(By.res("timeline_list")), 15_000)
            ?: error(
                "timeline_list not found. Ensure dev.ruurd.photos is signed in on the device, " +
                        "media permissions are granted, and the timeline is non-empty."
            )

    private fun MacrobenchmarkScope.continuousScroll(
        list: UiObject2,
        downSwipes: Int,
        upSwipes: Int,
        steps: Int
    ) {
        val bounds = list.visibleBounds
        val x = bounds.centerX()
        val bottomY = (bounds.bottom - bounds.height() * 0.15f).toInt()
        val topY = (bounds.top + bounds.height() * 0.15f).toInt()

        // Chain fast swipes with a brief sleep instead of waitForIdle() so momentum stays continuous.
        repeat(downSwipes) {
            device.swipe(x, bottomY, x, topY, steps)
            SystemClock.sleep(80)
        }
        if (upSwipes > 0) {
            SystemClock.sleep(250)
            repeat(upSwipes) {
                device.swipe(x, topY, x, bottomY, steps)
                SystemClock.sleep(80)
            }
        }
    }

    private fun mainActivityIntent(): Intent = Intent().apply {
        component = ComponentName(TARGET_PACKAGE, MAIN_ACTIVITY)
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
    }

    private companion object {
        const val TARGET_PACKAGE = "dev.ruurd.photos"
        const val MAIN_ACTIVITY = "dev.ruurd.photos.MainActivity"
    }
}