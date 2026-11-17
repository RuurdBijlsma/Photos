package dev.ruurd.photos.benchmark

import android.content.ComponentName
import android.content.Intent
import android.os.SystemClock
import androidx.benchmark.macro.junit4.BaselineProfileRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Until
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Records the baseline profile that ships as `app/src/main/baseline-prof.txt`.
 *
 * Needs a device signed in to `dev.ruurd.photos` with media permissions granted and a non-empty
 * timeline; the connected test uninstalls the app, so its data is gone afterwards. Run only this
 * class with `-PbaselineProfileRecording` (R8 renames app classes, and the committed profile has to
 * keep original names so R8 can rewrite them for minified builds), then copy the generated
 * `*_baseline-prof.txt` from `benchmark/build/outputs/connected_android_test_additional_output/`
 * into `app/src/main/`:
 *
 * ```
 * ./gradlew :benchmark:connectedBenchmarkAndroidTest \
 *   -Pandroid.testInstrumentationRunnerArguments.class=dev.ruurd.photos.benchmark.BaselineProfileGenerator \
 *   -PbaselineProfileRecording
 * ```
 */
@RunWith(AndroidJUnit4::class)
class BaselineProfileGenerator {

    @get:Rule
    val baselineProfileRule = BaselineProfileRule()

    @Test
    fun timelineBaselineProfile() = baselineProfileRule.collect(packageName = TARGET_PACKAGE) {
        pressHome()
        startActivityAndWait(mainActivityIntent())
        device.wait(Until.findObject(By.res("timeline_list")), 15_000)
        device.waitForIdle()

        val bounds = device.findObject(By.res("timeline_list")).visibleBounds
        val x = bounds.centerX()
        val topY = (bounds.top + bounds.height() * 0.15f).toInt()
        val bottomY = (bounds.bottom - bounds.height() * 0.15f).toInt()

        // First fling lands while startup scan/layout work is still running.
        repeat(4) {
            device.swipe(x, bottomY, x, topY, 14)
            SystemClock.sleep(80)
        }
        SystemClock.sleep(250)
        repeat(2) {
            device.swipe(x, topY, x, bottomY, 14)
            SystemClock.sleep(80)
        }
        device.waitForIdle()
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
