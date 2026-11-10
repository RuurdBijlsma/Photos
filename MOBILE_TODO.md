# MOBILE APP:

Support for Android 17 and up.

### Mobile stack:

Initial thoughts on libraries. not set in stone. Change if something better is available.

* UI: Jetpack Compose + Compose Material 3.
* Image Loading: Coil 3
    * (Supports Compose natively, coroutines-first, customizable disk caches, and handles both android.resource://,
      content:// MediaStore URIs, and HTTP/AVIF URLs).
* Local Database: Room
    * (with SQLite WAL mode for fast parallel reads/writes).
* Networking & Protobuf: Retrofit + OkHttp
    * with the official Java/Kotlin Protobuf lite runtime (compiled from your existing timeline.proto).
* Background Tasks: WorkManager (with CoroutineWorker) for auto-uploading and background synchronization.
* Video Playback: Jetpack Media3 (ExoPlayer).
* not sure yet

## CORE FEATURES

* auth (login, stay logged in)
* auto upload when a new photo/video is taken
* view timeline (including offline photos, in case no internet connection)
    * includes local vs cloud deduplication
    * show badge on timeline for cloud / local / uploading
    * scrub through timeline with scrubber on the right edge with floating date pill (similar to web frontend)
* view photo, pinch/double tap to zoom, share, add to album. generally native gallery gestures & interactions
* smooth UI feel, quick app start
* browse/view albums
* manually upload (TUS resumable upload, backend is already there)
* folder selection for what to backup. Default to DCIM/Camera, optionally let user add more folders
* cache timeline / other responses so opening app can be instant
* view timeline items offline
    * this means caching all cloud thumbnails? Investigate how to best handle this. Maybe that would be too much
      storage/downloading required. bounded LRU disk cache for thumbs?
    * For photos physically on the phone, load directly from Android’s local thumbs instead of querying the server.
* delete action, consider how to best do this, there's delete from device, delete from server, and soft delete (db sets
  deleted=true)
* in media-viewer
    * swipe up for info/gps/date pane
    * swipe down to return back to timeline

## LATER ON

* "Free up space" on phone button, deletes locally from phone from media items where it's detected it's already on
  server
* "cast" button
* semantic search (backend is already there), wouldnt need offline items to be searchable
* map view
* daily cards on front page (similar to cards in google photos), with notifications. Things like "On this day (1 year
  ago)", or "Cuddles and purrs" (cat pics).
* people page, where clustered faces are shown and can be labeled, and face clusters can be joined together.
* bin/trash, place where deleted items are, where they can be permanently deleted or restored to library.
* settings page - mostly app settings. on frontend this is theme settings, UI settings, concurrent uploads limit, album
  backup management, and missing media list (items that exist on DB but not on disk, can be (mass) deleted from db
  there). This is not all required for mobile. Mobile is more user focused rather that server management focused.
  Settings can include only sync on wifi, only while charging, not when roaming.
* User-friendly upload status UI
* set as wallpaper/lock screen option
* share photo to Ruurd Photos app to upload it (receive share intent)
* timeline
    * drag to select in timeline
        * long press to enter selection mode, the drag to select many items quickly.
    * pinch to change grid density
* connectivity indicator, on mobile bad internet is much more common. dont error on no internet, show indicators like (
  Connecting..., offline, etc.)

## BONUS, NOT REQUIRED

* motion photo viewer
* search offline items as well (may need design considerations early on, depens on added complexity / drawbacks)
* 3d panorama viewer
* explore page (backend endpoints are there already, just about showing the data. Some tables, histograms in nice UI,
  top visited places), includes location page where all photos/videos taken in that location are shown.\
* biometric lock for app? optionally of course

## ADMIN STUFF

even lower prio, dont want this in the app

* activity view, where user can see how the backend is progressing during ingest
* admin page - see background job queue, manage user accounts on server
* manage storage - see how full the server disk is, and get possibility to clear up storage by deleting large items


---

Phased implementation of CORE features

#### Phase 1: Foundation & Read-Only Timeline
* Set up Ktor/Retrofit with JWT storage in Android `EncryptedSharedPreferences` or `DataStore`.
* Compile `timeline.proto` to generate Kotlin/Java Protobuf models.
* Implement repository sync: fetch `GET /api/timeline/ratios` and `GET /api/timeline/by-month`, storing entries into Room.
* Query local `MediaStore` and merge them into the same Room table.
* Build the Compose timeline using `LazyVerticalGrid` with month headers and the draggable right-edge scrubber.

#### Phase 2: Media Viewer & Gestures
* Implement fullscreen viewer with swipe-between-items via `HorizontalPager`.
* Integrate `telephoto` / `zoomable` for double-tap and pinch-to-zoom.
* Media3 (ExoPlayer) integration for streaming videos or playing local video files directly from device storage.
* Add bottom action bar: Share (using native Android `Intent.ACTION_SEND`), Info panel (EXIF metadata), and Delete.

#### Phase 3: Backup Engine & Manual Upload
* UI screen for backup settings: enable backup, select folders (DCIM, WhatsApp, etc.), Wi-Fi only toggle.
* Room `upload_queue` table to track progress.
* `UploadWorker` implementation with TUS upload chunks.
* Timeline badge overlays: syncing spinner, local-only icon, or cloud checkmark.

#### Phase 4: Albums, Management & Deletion
* Album browsing and creating new albums.
* Drag-to-select and long-press multi-selection on the timeline.
* Deletion dialog: "Delete from Device", "Delete from Server", "Delete Everywhere".
* "Free up space" feature: find all items where `syncStatus == SYNCED`, count their total local file size, and prompt `createDeleteRequest` to purge them from the phone.

---

implement notes

* Compose LazyGrid Performance:
  * Timeline scrolling will stutter if items perform heavy formatting on the main thread. Ensure dates, timestamps, and URLs
  are pre-computed in Room or mapped once in the ViewModel, never inside the Compose @Composable item body. Pass stable
  Lambdas/IDs to LazyVerticalGrid.