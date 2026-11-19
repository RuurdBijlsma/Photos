## Quality checks & unit tests

Run formatting, Android Lint (including Compose rules), JVM unit tests, and debug/release/benchmark builds:

```powershell
./gradlew qualityCheck

# Individual checks:
./gradlew spotlessApply      # Auto-format Kotlin & Gradle scripts
./gradlew spotlessCheck      # Verify formatting
./gradlew :app:lintDebug     # Android Lint + compose-lint-checks
./gradlew :app:testDebugUnitTest
```

Check for performance regressions in timeline:

Ensure `dev.ruurd.photos` is signed in on the test device, media permissions are granted, and the
timeline is non-empty.

```powershell
./gradlew :benchmark:connectedBenchmarkAndroidTest

# When updating baseline:
./gradlew :benchmark:updatePerfBaseline

# Otherwise (check latest benchmark output against perf/baseline.json):
./gradlew :benchmark:checkPerfRegression

# Or run benchmarks and check in one command:
./gradlew :benchmark:benchmarkAndCheck
```

## Baseline profile (`app/src/main/baseline-prof.txt`)

Recorded from the timeline with `BaselineProfileGenerator`. Sign in and grant media permissions
first: the connected test uninstalls the app, so its data is gone afterwards.

R8 rewrites the committed original class names for minified variants, so the profile must be
recorded with `-PbaselineProfileRecording` (turns R8 off for the `benchmark` variant only; perf
benchmarks keep the minified build). Then copy the generated profile over the committed one:

```powershell
./gradlew :benchmark:connectedBenchmarkAndroidTest "-Pandroid.testInstrumentationRunnerArguments.class=dev.ruurd.photos.benchmark.BaselineProfileGenerator" -PbaselineProfileRecording
# -> benchmark/build/outputs/connected_android_test_additional_output/benchmark/connected/<device>/*-baseline-prof.txt
```
