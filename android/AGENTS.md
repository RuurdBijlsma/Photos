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

Lint runs with `warningsAsErrors`. Findings that are deliberate for a self-hosted client (plain
http / self-signed certs, dependency-version updates) and the app-level CompositionLocals are
listed with a reason in `app/lint.xml`; fix code instead of adding entries there.

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

## Startup profiling

Findings, numbers and rejected ideas live in `../docs/profiling-findings.md`. Read it before
changing anything on the startup path. Practical rules:

- The benchmark variant is the build to profile: minified, not debuggable, closest to release.
  `./gradlew :app:assembleBenchmark`, then
  `adb install -r -t app/build/outputs/apk/benchmark/app-benchmark.apk`. `-r -t` keeps the signed-in
  session; a `connectedBenchmarkAndroidTest` run uninstalls the app instead.
- `app/src/benchmark/AndroidManifest.xml` declares `<profileable android:shell="true" />`. Without
  it Perfetto captures no app-level `atrace` slices (`Choreographer#doFrame`, `Compose:*`) and the
  trace is useless. Do not remove it.
- Startup work runs in `PhotosApplication.onCreate` and `DefaultAppContainer`'s `lazy` fields.
  Keep new work out of `onCreate` unless it is measured to help; anything added there lands in
  `bindApplication` on every cold start.
- `androidx.startup` initializers from transitive dependencies run before `Application.onCreate`.
  Check the merged manifest (`app/build/intermediates/merged_manifest/benchmark/...`) before
  assuming a dependency is not initialized; remove unused ones with `tools:node="remove"` on the
  `androidx.startup.InitializationProvider` meta-data rather than by adding `tools:node="replace"`.
- Call `Activity.reportFullyDrawn()` only when real content is on screen. It is idempotent in the
  framework, so re-reporting is harmless, but reporting on a spinner makes TTFD meaningless.
- `timeToFullDisplayMs` is `null` until something calls `reportFullyDrawn()`. `null` there means
  "not measured", not "fast".
- Two consecutive benchmark runs differ by ~50 ms on this device (200 vs 253 ms median TTID for
  the same code). Judge a change against several runs, not one, and do not chase sub-50 ms deltas.
- `am start -W` is the quick loop (`adb shell am force-stop dev.ruurd.photos; adb shell am start -W
  -n dev.ruurd.photos/.MainActivity`); for TTID and TTFD together read `adb logcat -s
  ActivityTaskManager` and look for `Displayed … +Nms` / `Fully drawn … +Nms`. 10 samples is the
  minimum useful count.
- Perfetto callstack sampling does not pick up an app launched after the trace starts. Either use
  atrace slices, or start the trace with a `target_cmdline` filter and launch afterwards.
- `:benchmark:checkPerfRegression` compares the newest result against `perf/baseline.json` and can
  fail on the scroll/scrub benchmarks because the timeline content changed (a wiped database
  repopulated from MediaStore scrolls different images), not because of a code change. Check that
  the failing metric is one you touched before believing it.

## Networking & media data

- Retrofit converter order is `Scalars` → `Proto` → `kotlinx-serialization` (`photosApiConverterFactories()` in
  `di/AppContainer.kt`). kotlinx-serialization claims every type and throws for protobuf responses, and Retrofit 3
  resolves converters lazily on the first call, so a wrong order fails at runtime (sync notification), not at
  `create()`. `di/PhotosApiConvertersTest.kt` guards it.
- Photo endpoints key on the server's `cloudId`, never Room's surrogate `id`.
  `GET /api/photos/{cloudId}/download` answers `Content-Disposition: inline; filename="…"` and 404s when the file is
  gone.
- `DynamicBaseUrlInterceptor` rewrites scheme/host/port, so absolute URLs built from `sessionManager.getServerUrl()`
  work like relative ones.
- Full-res and share staging live in `full_res_cache/` / `share_cache/` under the files dir and are cleared on sign-out
  (`AppContainer.onClearSessionData`). `MediaFilePolicy` decides CLOUD_ONLY vs LOCAL_ONLY per item.

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
