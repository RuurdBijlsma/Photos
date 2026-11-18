### Part 5: Recommended Third-Party Libraries

* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.6.3` & `io.coil-kt.coil3:coil-network-okhttp:3.6.3` (Native
  Compose integration, coroutines, custom `MediaStoreThumbnailFetcher`, shared `OkHttpClient`, and AVIF decoding via
  `LibavifDecoder` + `ImageDecoder`; tune `LibavifDecoder` threads for single-hero `1440p` viewer decodes and keep an
  O(1) `LruCache` of `MemoryCache.Key`s from `onSuccess` rather than scanning `memoryCache.keys`).
* **Immutable Collections:** `org.jetbrains.kotlinx:kotlinx-collections-immutable:0.5.2` (`ImmutableList` for stable
  Compose row and pager skipping).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil3:0.19.0` (Coil 3 integration for Compose zoomable
  images with subsampling, Ultra HDR gainmaps, fling, inertia, and `HorizontalPager` cooperation; handle clicks via
  `ZoomableAsyncImage`'s `onClick`/`onLongClick` rather than `Modifier.clickable`, key the surrounding pager/slot, use
  `HorizontalPager(beyondViewportPageCount = 1)` with `settledPage`/zoom-gated full-res upgrades so fast swipes stay on
  lightweight previews, and verify Coil 3 `1440p.avif` disk-cache snapshot closure against `saket/telephoto#181`).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.11.1`,
  `androidx.media3:media3-ui-compose:1.11.1` (plus optionally `media3-ui-compose-material3:1.11.1`), &
  `androidx.media3:media3-datasource-okhttp:1.11.1` (Required so `ExoPlayer` shares `OkHttpClient` cookies and 401
  refresh; use `PlayerSurface` / `ContentFrame` with `rememberPresentationState(player, keepContentOnReset = ...)`
  only for same-item quality switches—resetting the shutter on `mediaId` change until `onRenderedFirstFrame`—tune
  `DefaultLoadControl` for fast remote start, and drive seekbar progress via state-read lambdas rather than legacy
  `media3-ui` Views).
* **JSON Serialization & Local EXIF:** `org.jetbrains.kotlin.plugin.serialization` +
  `org.jetbrains.kotlinx:kotlinx-serialization-json` + Retrofit `converter-kotlinx-serialization` (`2.11.0`) for
  type-safe `@Serializable` Navigation Compose routes and JSON endpoints (`/api/photos/{id}/item`, Phase 3–5 APIs),
  and `androidx.exifinterface:exifinterface:1.4.2` for offline local EXIF reading plus `LOCAL_ONLY` rotation and
  datetime editing.
* **Protobuf Compilation:** `com.google.protobuf:protobuf-javalite` with the Gradle Protobuf plugin to compile
  the `.proto` schemas.
* **Navigation:** `androidx.navigation:navigation-compose:2.10.2` (Type-safe Compose navigation routes via
  `@Serializable` classes; already configured in `gradle/libs.versions.toml`).
* **Local DB:** `androidx.room:room-runtime:2.8.5`, `androidx.room:room-ktx:2.8.5`, & KSP
  `androidx.room:room-compiler:2.8.5` (Configured with SQLite WAL mode for non-blocking concurrent reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.12.0` (WorkManager for background sync and TUS uploads. Note:
  verify foreground `setForeground(ForegroundInfo(..., FOREGROUND_SERVICE_TYPE_DATA_SYNC))` on `targetSdk = 37` devices
  for manual uploads—`2.12.0` has a reported `InvalidForegroundServiceTypeException` on newer targets; fall back to
  Android's User-Initiated Data Transfer (`UIDT`) API or `2.11.2` if reproduced).