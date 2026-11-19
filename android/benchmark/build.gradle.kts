import groovy.json.JsonOutput
import groovy.json.JsonSlurper
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.TaskAction
import java.io.File
import kotlin.math.ceil

plugins {
    alias(libs.plugins.android.test)
}

android {
    namespace = "dev.ruurd.photos.benchmark"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        minSdk = 35
        targetSdk = 37
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildTypes {
        create("benchmark") {
            isDebuggable = true
            signingConfig = signingConfigs.getByName("debug")
            matchingFallbacks += listOf("release")
        }
    }

    targetProjectPath = ":app"
    experimentalProperties["android.experimental.self-instrumenting"] = true
}

androidComponents {
    beforeVariants(selector().all()) { variant ->
        variant.enable = variant.buildType == "benchmark"
    }
}

dependencies {
    implementation(libs.androidx.junit)
    implementation(libs.androidx.espresso.core)
    implementation(libs.androidx.uiautomator)
    implementation(libs.androidx.benchmark.macro.junit4)
}

abstract class PerfRegressionTask : DefaultTask() {
    @get:Internal
    abstract val benchmarkOutputDir: DirectoryProperty

    @get:Internal
    abstract val baselineFile: RegularFileProperty

    @get:Input
    abstract val updateBaseline: Property<Boolean>

    @TaskAction
    fun run() {
        val outDir = benchmarkOutputDir.get().asFile
        if (!outDir.exists()) {
            throw GradleException("Benchmark output directory not found: ${outDir.absolutePath}")
        }

        val latestJson =
            outDir
                .walkTopDown()
                .filter { it.isFile && it.name.endsWith("-benchmarkData.json") }
                .maxByOrNull { it.lastModified() }
                ?: throw GradleException("No *-benchmarkData.json found under ${outDir.absolutePath}")

        val raw = JsonSlurper().parse(latestJson).asMap()
        val current = extractSummary(raw)

        println("Loaded benchmark results from: ${latestJson.absolutePath}")
        println(JsonOutput.prettyPrint(JsonOutput.toJson(current["benchmarks"])))

        val baselineTarget = baselineFile.get().asFile
        if (updateBaseline.get()) {
            baselineTarget.parentFile?.mkdirs()
            baselineTarget.writeText(JsonOutput.prettyPrint(JsonOutput.toJson(current)) + "\n")
            println("Updated baseline: ${baselineTarget.absolutePath}")
            return
        }

        val failures = mutableListOf<String>()
        val currentBenchmarks = current["benchmarks"].asMap()

        // Enforce P95 overrun budget on interactive scroll/scrub benchmarks (not cold startup).
        for ((name, rawMetrics) in currentBenchmarks) {
            if (name.startsWith("coldStartup")) continue
            val overrun = rawMetrics.asMap()["frameOverrunMs"]?.asMap() ?: continue
            val p95 = overrun["p95"].asDouble() ?: continue
            if (p95 > MAX_P95_OVERRUN_MS) {
                failures += "$name: frameOverrunMs P95 = ${"%.2f".format(p95)}ms > ${MAX_P95_OVERRUN_MS}ms budget"
            }
        }

        if (baselineTarget.exists()) {
            val baseline = JsonSlurper().parse(baselineTarget).asMap()
            val baselineBenchmarks = baseline["benchmarks"].asMap()

            for ((name, rawMetrics) in currentBenchmarks) {
                val curMetrics = rawMetrics.asMap()
                val baseMetrics = baselineBenchmarks[name]?.asMap() ?: continue

                val curCpu = curMetrics["frameDurationCpuMs"]?.asMap()
                val baseCpu = baseMetrics["frameDurationCpuMs"]?.asMap()
                if (curCpu != null && baseCpu != null) {
                    for ((pct, limit) in listOf(
                        "p50" to MAX_MEDIAN_REGRESSION,
                        "p95" to MAX_TAIL_REGRESSION,
                        "p99" to MAX_TAIL_REGRESSION,
                    )) {
                        val curVal = curCpu[pct].asDouble() ?: continue
                        val baseVal = baseCpu[pct].asDouble() ?: continue
                        val delta = curVal - baseVal
                        val ratio = if (baseVal > 0.0) delta / baseVal else 0.0
                        if (delta > MIN_ABS_DELTA_MS && ratio > limit) {
                            failures += "$name: frameDurationCpuMs $pct regressed from ${"%.2f".format(baseVal)}ms " +
                                "to ${"%.2f".format(curVal)}ms (+${"%.1f".format(ratio * 100.0)}%)"
                        }
                    }
                }

                val curStartup = curMetrics["timeToInitialDisplayMs"]?.asMap()
                val baseStartup = baseMetrics["timeToInitialDisplayMs"]?.asMap()
                if (curStartup != null && baseStartup != null) {
                    val curMedian = curStartup["median"].asDouble()
                    val baseMedian = baseStartup["median"].asDouble()
                    if (curMedian != null && baseMedian != null) {
                        val delta = curMedian - baseMedian
                        val ratio = if (baseMedian > 0.0) delta / baseMedian else 0.0
                        if (delta > MIN_STARTUP_ABS_DELTA_MS && ratio > MAX_MEDIAN_REGRESSION) {
                            failures += "$name: timeToInitialDisplayMs median regressed from ${"%.1f".format(baseMedian)}ms " +
                                "to ${"%.1f".format(curMedian)}ms (+${"%.1f".format(ratio * 100.0)}%)"
                        }
                    }
                }
            }
        } else {
            logger.warn("No baseline found at ${baselineTarget.absolutePath}; checked overrun budget only.")
        }

        if (failures.isNotEmpty()) {
            val msg =
                buildString {
                    appendLine("Performance regression gate FAILED:")
                    failures.forEach { appendLine("  - $it") }
                }
            throw GradleException(msg)
        }

        println("Performance regression gate PASSED.")
    }

    private fun extractSummary(raw: Map<String, Any?>): Map<String, Any?> {
        val context = raw["context"].asMap()["build"].asMap()
        val benchmarksOut = linkedMapOf<String, Any?>()

        for (entry in raw["benchmarks"].asList()) {
            val b = entry.asMap()
            val name = b["name"] as? String ?: continue
            val sampled = b["sampledMetrics"].asMap()
            val metrics = b["metrics"].asMap()

            benchmarksOut[name] =
                linkedMapOf(
                    "frameDurationCpuMs" to sampled["frameDurationCpuMs"]?.asMap()?.let(::summarizeSampled),
                    "frameOverrunMs" to sampled["frameOverrunMs"]?.asMap()?.let(::summarizeSampled),
                    "timeToInitialDisplayMs" to metrics["timeToInitialDisplayMs"]?.asMap()?.let(::summarizeScalar),
                    "timeToFullDisplayMs" to metrics["timeToFullDisplayMs"]?.asMap()?.let(::summarizeScalar),
                )
        }

        return linkedMapOf(
            "context" to context,
            "benchmarks" to benchmarksOut,
        )
    }

    private fun summarizeSampled(metric: Map<String, Any?>): Map<String, Double> {
        val p50 = metric["P50"].asDouble()
        val p95 = metric["P95"].asDouble()
        val p99 = metric["P99"].asDouble()
        if (p50 != null && p95 != null && p99 != null) {
            return linkedMapOf(
                "p50" to p50,
                "p90" to (metric["P90"].asDouble() ?: p95),
                "p95" to p95,
                "p99" to p99,
            )
        }

        val all =
            metric["runs"]
                .asList()
                .flatMap { it.asList() }
                .mapNotNull { it.asDouble() }
                .sorted()

        return linkedMapOf(
            "p50" to nearestRank(all, 50.0),
            "p90" to nearestRank(all, 90.0),
            "p95" to nearestRank(all, 95.0),
            "p99" to nearestRank(all, 99.0),
        )
    }

    private fun summarizeScalar(metric: Map<String, Any?>): Map<String, Double> {
        val runs = metric["runs"].asList().mapNotNull { it.asDouble() }.sorted()
        val median = metric["median"].asDouble() ?: nearestRank(runs, 50.0)
        val min = metric["minimum"].asDouble() ?: (runs.firstOrNull() ?: 0.0)
        val max = metric["maximum"].asDouble() ?: (runs.lastOrNull() ?: 0.0)
        return linkedMapOf(
            "min" to min,
            "median" to median,
            "max" to max,
        )
    }

    private fun nearestRank(
        sorted: List<Double>,
        p: Double,
    ): Double {
        if (sorted.isEmpty()) return 0.0
        val idx = (ceil((p / 100.0) * sorted.size).toInt() - 1).coerceIn(0, sorted.lastIndex)
        return sorted[idx]
    }

    @Suppress("UNCHECKED_CAST")
    private fun Any?.asMap(): Map<String, Any?> = (this as? Map<String, Any?>) ?: emptyMap()

    @Suppress("UNCHECKED_CAST")
    private fun Any?.asList(): List<Any?> = (this as? List<Any?>) ?: emptyList()

    private fun Any?.asDouble(): Double? = (this as? Number)?.toDouble()

    private companion object {
        const val MAX_MEDIAN_REGRESSION = 0.15
        const val MAX_TAIL_REGRESSION = 0.20
        const val MIN_ABS_DELTA_MS = 0.5
        const val MIN_STARTUP_ABS_DELTA_MS = 15.0
        const val MAX_P95_OVERRUN_MS = 4.0
    }
}

val benchmarkJsonOutputDir =
    layout.buildDirectory.dir(
        "outputs/connected_android_test_additional_output/benchmark/connected",
    )
val perfBaselineFile = rootProject.layout.projectDirectory.file("perf/baseline.json")

val checkPerfRegression by tasks.registering(PerfRegressionTask::class) {
    group = "verification"
    description = "Checks the latest benchmark output against perf/baseline.json and overrun budgets."
    benchmarkOutputDir.set(benchmarkJsonOutputDir)
    baselineFile.set(perfBaselineFile)
    updateBaseline.set(false)
    outputs.upToDateWhen { false }
}

val updatePerfBaseline by tasks.registering(PerfRegressionTask::class) {
    group = "verification"
    description = "Writes the latest benchmark output to perf/baseline.json."
    benchmarkOutputDir.set(benchmarkJsonOutputDir)
    baselineFile.set(perfBaselineFile)
    updateBaseline.set(true)
    outputs.upToDateWhen { false }
}

tasks.register("benchmarkAndCheck") {
    group = "verification"
    description = "Runs connectedBenchmarkAndroidTest and verifies no performance regressions."
    dependsOn("connectedBenchmarkAndroidTest", checkPerfRegression)
}

checkPerfRegression.configure {
    mustRunAfter("connectedBenchmarkAndroidTest")
}
