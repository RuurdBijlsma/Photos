# Android startup profiling — findings

Target: `android/` app (`dev.ruurd.photos`) on a Pixel 8 Pro (husky, Android API 37), wireless ADB.
Goal: make cold start faster and prove it with measurements.

## How it was measured

| Source                              | What it gives                                                                           | Command                                                                                             |
|-------------------------------------|-----------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------|
| `adb shell am start -W`             | TTID per launch, cheap loop                                                             | `adb shell am force-stop dev.ruurd.photos; adb shell am start -W -n dev.ruurd.photos/.MainActivity` |
| `adb logcat -s ActivityTaskManager` | `Displayed … +Nms` (TTID) and `Fully drawn … +Nms` (TTFD)                               | —                                                                                                   |
| Macrobenchmark (`:benchmark`)       | `timeToInitialDisplayMs`, `timeToFullDisplayMs`, `frameDurationCpuMs`, `frameOverrunMs` | `./gradlew :benchmark:connectedBenchmarkAndroidTest`                                                |
| Perfetto trace + `trace_processor`  | per-slice breakdown, absolute phase timings                                             | see "Tracing" below                                                                                 |

Profiling build: the `benchmark` variant (minified, not debuggable, closest to release), installed
over the existing app with `adb install -r -t` so the signed-in state survived.

## Startup anatomy (measured, not estimated)

From a Perfetto trace, timestamps relative to the start of `bindApplication` (ms):

|     t | Slice                                               |      dur |
|------:|-----------------------------------------------------|---------:|
|   0.0 | `bindApplication`                                   | **20.1** |
|   3.4 | └ `OpenDexFilesFromOat`                             |      3.2 |
|  11.8 | └ `makeApplication`                                 |      1.1 |
|  13.0 | └ `Startup` (androidx.startup initializers)         |      2.5 |
|  20.3 | `clientTransactionExecuted`                         | **25.6** |
|  20.3 | └ `activityStart` (incl. `performCreate` 3.6)       |      9.5 |
|  30.8 | └ `activityResume` (incl. `performResume` 4.8)      |     15.1 |
|  46.0 | `Choreographer#doFrame` **first frame**             | **66.3** |
| 112.4 | second `Choreographer#doFrame` (rows laid out)      | **29.9** |
| 183.0 | `reportFullyDrawn()` (first real content on screen) |        — |

So a cold start to first frame is ~112 ms of which ~96 ms is two frames of Compose work. Inside
the first frame: `Compose:recompose` 23–33 ms, `measure` 8–16 ms, `draw-VRI` 8 ms,
`Compose:applyChanges` 3 ms, `Compose:initializeView` 3 ms.

Conclusions:

- **The first two Compose frames are the whole app-side cost.** Nothing else comes close.
- **`bindApplication` is only ~20 ms**, of which initializers are ~2.5 ms and dex loading ~3 ms.
  This is the cheap place to delete work, but the ceiling is low.
- **Class loading is not a problem.** Class-init slices inside `recompose` are microseconds, so
  eager construction of OkHttp/TLS/Retrofit is not worth attacking.
- **Roughly half of TTID is not app frame work**: process start, `bindApplication`,
  `clientTransactionExecuted` (window/surface setup) and the system splash.

## Baseline numbers

Macrobenchmark, `coldStartupAndScroll`, 5 iterations, before any change:

| Metric                   |   min |    median |   max |    CV |
|--------------------------|------:|----------:|------:|------:|
| `timeToInitialDisplayMs` | 235.0 | **253.3** | 365.6 | 0.215 |
| `frameCount`             |   172 |       177 |   181 | 0.022 |

The committed `perf/baseline.json` from an earlier run of the same benchmark says TTID median
200.1 (min 194.7, max 285.9). **That ~50 ms spread between two runs of identical code is the
noise floor of this measurement on this device.** Do not chase deltas smaller than that with the
macrobenchmark; use many `am start -W` samples instead.

`timeToFullDisplayMs` was `null` everywhere because the app never called `reportFullyDrawn()`.

## Changes made, and what each one measured

### 1. `reportFullyDrawn()` on first real timeline content

`features/timeline/TimelineScreen.kt`. Fires after the first frame that actually contains rows,
not the spinner, so TTFD is now a real number (419 ms median in the macrobenchmark, 233 ms via
logcat). This is measurement infrastructure, not a speedup. `reportFullyDrawn()` is idempotent in
the framework, so repeated calls are harmless.

### 2. Removed the `androidx.emoji2` startup initializer

`app/src/main/AndroidManifest.xml`. Nothing uses `EmojiCompat`; it comes in transitively and its
initializer runs inside `bindApplication` on every cold start. Measured directly from traces:

|        | `EmojiCompatInitializer` |
|--------|-------------------------:|
| before |   2.575, 0.832, 0.980 ms |
| after  |              not present |

~1 ms per cold start (median 0.98). Small, but it is free.

### 3. Open the Room database at process start, off the main thread

`di/AppContainer.kt` (`warmUp()`), called from `PhotosApplication.onCreate`. The first timeline
paint needs a Room open + stage-1 query, which previously could only start after the DataStore
session load that gates the UI. Warming it up overlaps the two.

Measured A/B, 8 cold starts per variant, tracing identical, same data on device. Metric = time
from `bindApplication` start to the `reportFullyDrawn()` slice (time to content):

|                  | samples (ms)                                           |    median |   max |
|------------------|--------------------------------------------------------|----------:|------:|
| without `warmUp` | 199.7, 202.8, 180.1, 222.2, 179.0, 230.2, 192.0, 182.4 | **195.8** | 230.2 |
| with `warmUp`    | 183.0, 196.7, 181.1, 184.2, 188.1, 178.2, 186.4, 188.3 | **185.3** | 196.7 |

~10 ms faster to content, and the slow tail is much tighter. TTID is unaffected — 10 clean
`am start -W` samples per variant gave median 173 ms with `warmUp` and 177 ms without, i.e. the
same within noise.

## Things considered and rejected

- **Deferring the first-frame list composition** — the first frame *is* where the wall time is;
  deferring it keeps the splash up longer.
- **Removing the `NavHost`** — 23–33 ms of recompose is spread over theme, nav, main screen and
  timeline; nav is a small slice of it and removing it is a large refactor.
- **`LazyLayoutCacheWindow` tuning** — affects scrolling, not startup.
- **Deferring `TimelineProjectionStore` stage 2** — already debounced 150 ms and off the critical
  path.
- **Eagerly constructing OkHttp/Retrofit/TLS** — class-init slices are microseconds; not the cost.

## What would actually be worth doing next

The remaining app-side cost is the 66 ms first composition of theme + `NavHost` + `MainScreen` +
timeline, and the 30 ms second frame that lays out the rows. Getting a real win means reducing
what the first composition has to build (e.g. a dedicated splash-compatible lightweight first
screen instead of the full app shell), not shaving initializers. That is a refactor, not a tweak.

## Tracing

`adb shell perfetto --txt -c <config> -o /data/misc/perfetto-traces/<name>`, start it, then
`am force-stop` + `am start` the app while it records. Analyze with the `perfetto` Python package:

```python
from perfetto.trace_processor import TraceProcessor

tp = TraceProcessor(trace="x.perfetto-trace")
for r in tp.query("""
  select s.name, s.dur/1e6 as ms, s.depth
  from slice s join thread_track tt on s.track_id=tt.id
  join thread th on tt.utid=th.utid join process p on th.upid=p.upid
  where p.name='dev.ruurd.photos' and th.is_main_thread=1 and s.dur > 0
  order by s.ts limit 60"""):
    print(r.ms, r.name)
```

`slice.dur` is **nanoseconds** — divide by 1e6 for ms.

## Gotchas that cost time

- The app must be `profileable` for Perfetto to capture app-level `atrace` slices. `debuggable`
  alone is not enough, and the benchmark build is not debuggable. `app/src/benchmark/AndroidManifest.xml`
  declares `<profileable android:shell="true" />`; without it the app process shows no
  `Choreographer#doFrame` / `Compose:*` slices at all.
- `adb install -r -t` keeps app data (and the signed-in session). A `connectedBenchmarkAndroidTest`
  run **uninstalls the app**, so the session and the Room database are gone afterwards — and a
  freshly installed app starts with an empty database, which makes its first-start timings
  incomparable to a warm one. Sign in / grant permissions again before measuring.
- Right after install the app is dexopt `verify` (`reason=install`): no AOT compilation, baseline
  profile only. Numbers taken immediately after installing are pessimistic.
- `perfetto --txt -c … --background-wait` with a shell `&` produced a 0-byte trace. Run the
  recorder in the foreground of a background job instead.
- Perfetto callstack sampling (`linux.perf` / `traced_perf`) did not pick up an app launched
  *after* the trace started: `target_cmdline` yielded ~0 samples, and `scope: SCOPE_APP` is not a
  valid config field. Use atrace slices, or start the trace with a `target_cmdline` filter and
  launch afterwards.
- `:benchmark:checkPerfRegression` compares the newest result against `perf/baseline.json`. It can
  fail on the scroll/scrub benchmarks purely because the device's timeline content changed (a
  wiped database repopulated from MediaStore scrolls different images). Check whether the failing
  metric is one you touched before believing it.
