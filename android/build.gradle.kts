// Top-level build file where you can add configuration options common to all sub-projects/modules.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.test) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.serialization) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.protobuf) apply false
    alias(libs.plugins.spotless)
}

spotless {
    kotlin {
        target("app/src/**/*.kt", "benchmark/src/**/*.kt")
        targetExclude("**/build/**")
        ktlint()
            .editorConfigOverride(
                mapOf(
                    "ktlint_function_naming_ignore_when_annotated_with" to "Composable",
                ),
            )
    }
    kotlinGradle {
        target("*.kts", "app/*.kts", "benchmark/*.kts")
        ktlint()
    }
}

tasks.register("qualityCheck") {
    group = "verification"
    description = "Runs formatting, Android lint, JVM unit tests, and debug/release/benchmark builds."
    dependsOn(
        "spotlessCheck",
        ":app:lintDebug",
        ":app:testDebugUnitTest",
        ":app:assembleDebug",
        ":app:assembleRelease",
        ":benchmark:compileBenchmarkKotlin",
    )
}