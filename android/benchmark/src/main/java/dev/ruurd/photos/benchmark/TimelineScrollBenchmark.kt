package dev.ruurd.photos.benchmark

import android.content.ComponentName
import android.content.Intent
import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TimelineScrollBenchmark {

    @get:Rule
    val benchmarkRule = MacrobenchmarkRule()

    @Test
    fun scrollTimelineFling() = benchmarkRule.measureRepeated(
        packageName = TARGET_PACKAGE,
        metrics = listOf(FrameTimingMetric()),
        compilationMode = CompilationMode.Full(),
        startupMode = StartupMode.WARM,
        iterations = 8,
        setupBlock = { launchBenchmarkTimeline() }
    ) {
        val list = device.wait(Until.findObject(By.res("timeline_list")), 10_000)
            ?: error("timeline_list not found")
        list.setGestureMargin(device.displayWidth / 5)

        repeat(4) {
            list.fling(Direction.DOWN)
            device.waitForIdle()
        }
        repeat(2) {
            list.fling(Direction.UP)
            device.waitForIdle()
        }
    }

    @Test
    fun scrubTimeline() = benchmarkRule.measureRepeated(
        packageName = TARGET_PACKAGE,
        metrics = listOf(FrameTimingMetric()),
        compilationMode = CompilationMode.Full(),
        startupMode = StartupMode.WARM,
        iterations = 8,
        setupBlock = { launchBenchmarkTimeline() }
    ) {
        val scrubber = device.wait(Until.findObject(By.res("timeline_scrubber")), 10_000)
            ?: error("timeline_scrubber not found")
        val bounds = scrubber.visibleBounds
        val x = bounds.centerX()
        val topY = bounds.top + (bounds.height() * 0.15f).toInt()
        val bottomY = bounds.top + (bounds.height() * 0.85f).toInt()

        device.swipe(x, topY, x, bottomY, 45)
        device.waitForIdle()
        device.swipe(x, bottomY, x, topY, 45)
        device.waitForIdle()
    }

    private fun androidx.benchmark.macro.MacrobenchmarkScope.launchBenchmarkTimeline() {
        pressHome()
        val intent = Intent().apply {
            component = ComponentName(TARGET_PACKAGE, BENCHMARK_ACTIVITY)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        }
        startActivityAndWait(intent)
        device.wait(Until.hasObject(By.res("timeline_list")), 10_000)
    }

    private companion object {
        const val TARGET_PACKAGE = "dev.ruurd.photos"
        const val BENCHMARK_ACTIVITY = "dev.ruurd.photos.BenchmarkTimelineActivity"
    }
}
