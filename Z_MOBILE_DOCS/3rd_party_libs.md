### Part 5: Recommended Third-Party Libraries

* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.6.3` & `io.coil-kt.coil3:coil-network-okhttp:3.6.3` (Native
  Compose integration, coroutines, custom `MediaStoreThumbnailFetcher`, shared `OkHttpClient`, and AVIF decoding via
  `LibavifDecoder` + `ImageDecoder`).
* **Immutable Collections:** `org.jetbrains.kotlinx:kotlinx-collections-immutable:0.5.2` (`ImmutableList` for stable
  Compose row skipping).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil3:0.19.0` (Coil 3 integration for Compose zoomable
  images with subsampling, Ultra HDR gainmaps, fling, inertia, and `HorizontalPager` cooperation).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.11.1`,
  `androidx.media3:media3-ui-compose:1.11.1` (plus optionally `media3-ui-compose-material3:1.11.1`), &
  `androidx.media3:media3-datasource-okhttp:1.11.1` (Required so `ExoPlayer` shares `OkHttpClient` cookies and 401
  refresh; use `PlayerSurface` / `ContentFrame` rather than legacy `media3-ui` Views).
* **JSON Serialization & Local EXIF:** `org.jetbrains.kotlin.plugin.serialization` +
  `org.jetbrains.kotlinx:kotlinx-serialization-json` + Retrofit `converter-kotlinx-serialization` (`2.11.0`) for
  type-safe `@Serializable` Navigation Compose routes and JSON endpoints (`/api/photos/{id}/item`, Phase 3–5 APIs),
  and `androidx.exifinterface:exifinterface:1.4.2` for offline local EXIF reading and `LOCAL_ONLY` rotation.
* **Protobuf Compilation:** `com.google.protobuf:protobuf-javalite` with the Gradle Protobuf plugin to compile
  the `.proto` schemas.
* **Navigation:** `androidx.navigation:navigation-compose:2.10.2` (Type-safe Compose navigation routes via
  `@Serializable` classes).
* **Local DB:** `androidx.room:room-runtime:2.8.5`, `androidx.room:room-ktx:2.8.5`, & KSP
  `androidx.room:room-compiler:2.8.5` (Configured with SQLite WAL mode for non-blocking concurrent reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.12.0` (WorkManager for background sync and TUS uploads).