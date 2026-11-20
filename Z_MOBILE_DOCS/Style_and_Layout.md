## Purpose

Visual and interaction style guide for the native Android app (Jetpack Compose + Material 3, Android 15+ / API 35+,
`targetSdk = 37`).

The goal is to preserve the distinctive identity of the mobile web UI—**large concentric border radii, month-rounded
photo blocks, and the floating pill navigation bar with a detached Search button**—while adapting surfaces, motion, and
system integration so the app feels unmistakably native on Android.

*(All `dp`, `sp`, and spring values below are intentional starting defaults meant to be tuned on-device once rendered.)*

---

## 1. Color & Surface System

### 1.1 Dynamic Color & Opaque Surfaces

* **Theme Engine:** Start with Android `dynamicLightColorScheme` and `dynamicDarkColorScheme` (with optional
  MaterialKolor theme engine support later).
* **No Blurred Wallpaper Background:** Unlike the web app (`MainLayout.vue`), do **not** place a blurred photo behind
  the entire app scaffold. Use clean, **opaque** Material 3 tonal surfaces (`background`, `surface`,
  `surfaceContainer*`).
* **Dark Mode:** Use standard Material 3 dark surfaces (`surface` / `surfaceContainerLow`) rather than pure `#000000`,
  except inside the fullscreen media viewer.
* **Media Viewer Exception:** `MediaViewerScreen` uses true black (`#000000`) as its backdrop when resting, fading
  its backdrop alpha proportionally during interactive swipe-down dismiss so `TimelineScreen` shows through behind it.

### 1.2 Semantic Colors (`LocalSemanticColors`)

Keep non-theme-role status colors in a `CompositionLocal` (`LocalSemanticColors`) for consistent status communication
across light and dark modes:

* `offline` / warning tint (amber/muted neutral for offline indicator pill)
* `syncing` (active upload / sync accent)
* `localOnly` (subtle badge tint for items not yet backed up)
* `error` (upload failed / server unreachable, mapped from M3 `error` / `errorContainer`)

---

## 2. Typography

Match the web client’s pairing of **Montserrat** (display/brand headings) and **Roboto** (Android system UI & dense
text):

| Role                      | Font Family                    | Weight & Size (Starting Defaults)        | Used For                                                           |
|---------------------------|--------------------------------|------------------------------------------|--------------------------------------------------------------------|
| **Month Header**          | Montserrat                     | `SemiBold`, `22.sp`–`24.sp`              | `"November"`, `"October"` section titles                           |
| **Month Header Year**     | Montserrat                     | `Regular`/`Medium`, `18.sp`, `70%` alpha | Non-current year beside month name (e.g., `"2025"`)                |
| **Card Title**            | Montserrat                     | `SemiBold`, `18.sp`–`20.sp`              | Daily Card titles (`"On This Day"`), featured headers              |
| **Screen / Sheet Title**  | Montserrat                     | `SemiBold`, `20.sp`–`22.sp`              | Top-level sheet titles, empty states, `"Collections"`              |
| **Scrubber Tooltip**      | Montserrat                     | `SemiBold`, `12.sp`                      | Floating pill label while scrubbing (`"Nov 2026"`)                 |
| **Nav & Pill Labels**     | Roboto                         | `Medium`/`SemiBold`, `13.sp`–`14.sp`     | Bottom bar tabs (`Photos`, `Collections`, `Explore`), status pills |
| **Body & Subtitles**      | Roboto                         | `Regular`, `14.sp`–`16.sp`               | Card subtitles, settings descriptions, metadata labels             |
| **Technical / Durations** | Roboto (`tnum`) or Roboto Mono | `SemiBold`, `11.sp`–`12.sp`              | Video duration (`0:14`), EXIF values, file sizes                   |

---

## 3. Concentric Shape System & Corner Radii

### 3.1 The Concentric Rule

Large rounded corners are a core visual signature. Whenever a rounded element sits padded inside a rounded parent,
derive the inner radius from the outer radius so curves track cleanly:

$$\text{innerRadius} = \max(\text{minRadius},\; \text{outerRadius} - \text{padding})$$

```kotlin
fun concentricRadius(outer: Dp, padding: Dp, min: Dp = 8.dp): Dp =
    (outer - padding).coerceAtLeast(min)
```

### 3.2 Shape Token Scale (Starting Defaults)

| Token / Component                 | Corner Radius                       | Notes                                                                                                                           |
|-----------------------------------|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------|
| **Pills & Circular Buttons**      | `CircleShape` (`50%`)               | Floating bottom nav bar, active tab indicator, detached Search button, connectivity pill, viewer action pills, scrubber tooltip |
| **Modal Bottom Sheets & Dialogs** | `28.dp`–`32.dp`                     | Top corners on bottom sheets (Avatar menu, Info/EXIF sheet, Delete confirmation); all corners on dialogs                        |
| **Content Cards & Month Blocks**  | `24.dp`                             | Month photo grid outer corners, Daily Cards, Album cards, Device Folder cards                                                   |
| **Nested Thumbnails / Sub-cards** | `16.dp`                             | E.g. `24.dp` card with `8.dp` padding, or `28.dp` sheet with `12.dp` padding                                                    |
| **Grouped Settings / List Rows**  | `20.dp` outer / `4.dp`–`8.dp` inner | Expressive segmented card groups (first/last items rounded `20.dp`, middle gaps `2.dp`–`4.dp`)                                  |
| **Inner Timeline Grid Items**     | `0.dp`                              | Sharp corners between adjacent grid items (`2.dp` gap)                                                                          |
| **In-Grid Micro Overlays**        | `6.dp`                              | Video duration pill in top-right of grid tile                                                                                   |

---

## 4. Blur & Elevation Policy

* **General UI (Opaque by Default):** Keep navigation bars, cards, sheets, and dialogs **opaque** using Material 3 tonal
  hierarchy (`surfaceContainerLow` → `surfaceContainerHigh`) plus soft ambient shadow where floating. Avoid full-screen
  or nav-bar backdrop blur so the app looks crisp, native, and avoids GPU overdraw.
* **Media Viewer Floating Pills:** Inside `MediaViewerScreen`, use crisp dark translucent scrims
  (`Color.Black.copy(alpha = 0.45f)`–`0.6f`) with subtle borders for floating pill action bars.

---

## 5. Motion, Transitions & Haptics

* **Shared-Element Photo Transitions & Visible Timeline Behind Drag:** Host `MediaViewerScreen` in a
  `SharedTransitionLayout` / translucent overlay above `NavHost` rather than an opaque `NavHost` `composable`
  destination (which hides/unmounts the previous destination once entered). Because `MainScreen` stays mounted inside
  `NavHost` while the overlay renders above it, coordinate shared-bounds visibility with
  `sharedElementWithCallerManagedVisibility` (or explicit rect/bounds capture), suppress accessibility and focus on the
  underlying `NavHost` container while the viewer is open, and expose a hoisted scroll-sync bridge to `TimelineScreen`'s
  `timelineListState`:
    * Keep `sharedElement` modifiers inactive/bypassed during `isScrubbing` (or attach only for the active viewer item)
      so fast timeline scrubbing incurs zero shared-element registration overhead (verify against
      `TimelineScrollBenchmark.kt`).
    * Tapping a photo/video in the timeline grid transitions via shared-bounds / container transform into
      `MediaViewerScreen`, and dragging down interactively shrinks the viewer over the live, visible timeline behind it.
    * Rather than scrolling `TimelineScreen` on every fast swipe *or* calling `listState.scrollToItem(index)` on frame 1
      of swipe-down dismiss (which forces synchronous row/tile composition and thumbnail IPC right under the drag
      finger), silently scroll `timelineListState` ~250ms after `settledPage` pauses whenever `activeMediaId` is
      offscreen. If the user swipes and immediately dismisses mid-fling before the settle sync fires, fall back to a
      scale-to-center + fade dismiss and sync `TimelineScreen`'s scroll position as dismiss settles.
    * Extend `BaselineProfileGenerator` to exercise viewer open, fast `HorizontalPager` swiping, pinch/double-tap zoom,
      and swipe-down dismiss so those hot paths are AOT-compiled in release builds.
* **Spring Physics:** Use snappy, natural Material 3 Expressive springs (avoid sluggish linear/ease curves):
    * Spatial changes (shared photo bounds, bottom sheet expand, viewer swipe-to-dismiss): lightly damped spring (
      `DampingRatioNoBouncy` or very slight `0.85f` with `StiffnessMediumLow` / `StiffnessMedium`).
    * Micro-interactions (selection scale-down, active tab pill slide, badge pop-in): crisp spring (`StiffnessMedium`,
      minimal overshoot).
* **Restrained Haptics:** Use `LocalHapticFeedback` sparingly on meaningful tactile moments only:
    * **Long-press to enter drag-select** on the timeline grid (`LongPress`).
    * **Year boundary crossings** while dragging the right-edge `TimelineScrubber` (`ContextClick` / tick at
      **year** granularity, not every month, to avoid buzz fatigue).

---

## 6. Screen & Component Specifications

### 6.1 Top App Bar, Connectivity Pill & Avatar Menu

* **Layout:** Edge-to-edge header sitting above the timeline content (with a subtle gradient scrim under the status bar
  when scrolled away):
    * **Left:** Colorful flower-petal Ruurd Photos logo mark (`32.dp`, rotates with pull-to-refresh and active sync).
    * **Center:** **Connectivity / Sync Status Pill** (`CircleShape`, height `30.dp`). Shown when pulling to refresh or
      when state is `Offline`, `Syncing...`, `Server unreachable`, or `Sync failed`; hidden when idle/up-to-date so the
      header stays clean.
    * **Right:** Circular **User Avatar** (`38.dp`, `primaryContainer` background with user initial `"R"` or profile
      photo).
* **Scroll Behavior:** The top bar collapses/slides away on scroll down and returns on scroll up, letting the photo
  timeline flow cleanly beneath the status bar scrim.
* **Avatar Menu:** Tapping the avatar opens a `30.dp` top-rounded modal bottom sheet hosting:
    * User profile & server URL / connection status + manual `Sync Library Now` & `Sign Out`
    * Backup status & **Device Folders ("On this device")** *(Phase 3)*
    * **Free up space** *(Phase 4)*
    * **Backup Settings**, **App Settings** (theme, grid density), **Trash / Bin** *(when implemented)*

### 6.2 Floating Bottom Navigation Bar + Detached Search Button

* **Visibility:** Always anchored at the bottom above `WindowInsets.navigationBars` (with ~`12.dp`–`16.dp` margin).
  **Does not hide on scroll down.** List content scrolls behind it (add bottom `contentPadding` to lists so the final
  row can scroll above the bar).
* **Main Nav Pill:**
    * Shape: `CircleShape` (stadium pill), opaque `surfaceContainerHigh` with subtle tonal elevation/shadow (`6.dp`–
      `8.dp`).
    * **Phase 1 Destinations:** `Photos` | `Collections` | `Explore` (unimplemented pages render clean placeholder
      screens).
    * **Tab Styling (Web Parity):**
        * **Active tab:** Nested concentric `CircleShape` pill (`primaryContainer` fill) containing **Icon + Text label
          ** (`onPrimaryContainer`).
        * **Inactive tabs:** **Text label only** (`onSurfaceVariant`), no icon—preserving the clean, airy proportions of
          the screenshot.
* **Detached Search Button:**
    * Shape: `CircleShape`, matching the height of the floating nav pill (`56.dp`), opaque `surfaceContainerHigh` with a
      search magnifying-glass icon, spaced `10.dp` to the right of the nav pill.
    * **Route Rule:** When navigating into `/search`, the detached Search icon-button **disappears** (animated
      scale/fade out).

### 6.3 Daily Cards Carousel *(Future / Defined Look)*

* **Placement:** Full-bleed horizontal `LazyRow` at the top of the timeline above the first month header, with `16.dp`
  horizontal start/end padding and `10.dp`–`12.dp` item spacing so the next card peeks in from the right.
* **Card Geometry:** Portrait aspect ratio (~`2:3` or `3:4`, width `~150.dp`–`170.dp`), `24.dp` corner radius.
* **Overlays:**
    * Bottom vertical dark gradient scrim (`Transparent` → `Color.Black.copy(alpha = 0.72f)`) for legible white text.
    * Title in **Montserrat SemiBold** (`"On This Day"`, `"Location Estimatr"`), 1-line subtitle in **Roboto** (
      `"11 years ago"`, `"Where was this taken?"`).
    * Subtle `×` dismiss icon in the top-right (`White.copy(alpha = 0.7f)`).

### 6.4 Timeline Grid, Month Grouping & Tile Overlays

* **Month Headers:**
    * Padded generously (`horizontal = 24.dp`, `top = 16.dp`, `bottom = 10.dp`).
    * Month name in **Montserrat SemiBold**; if `year != currentYear`, show the year beside it with `16.dp` spacing in
      muted `onSurface.copy(alpha = 0.7f)`.
* **Justified Rows & Month-Block Clipping (Web `VirtualRow.vue` Parity):**
    * `2.dp` horizontal and vertical gaps between items/rows (`ITEM_GAP = 2.dp`).
    * **No corner rounding (`0.dp`) on inner photo/video tiles.**
    * **Outer month block corners (`24.dp`):**
        * First row of a month: clip top-left and top-right corners to `24.dp`.
        * Last row of a month: clip bottom-left and bottom-right corners to `24.dp`.
        * Single-row month: all four outer corners clipped to `24.dp`.
* **Tile Corner Overlays (Restrained Photo Wall):**
    * **Top-Right — Video Indicator:** Dark translucent pill (`Color.Black.copy(alpha = 0.6f)`, `6.dp` radius,
      `horizontal = 6.dp, vertical = 2.dp` padding) with duration text (`0:14`) + play icon (`12.dp`) in white.
    * **Bottom-Right — Sync Status Badge:** Keep normal browsing clean:
        * `SYNCED`: **No badge** (normal state).
        * `CLOUD_ONLY`: **No badge** (normal state).
        * `LOCAL_ONLY` (not backed up): Subtle compact icon (`cloud_off` / device-only indicator inside a small soft
          dark pill).
        * `UPLOADING` / `SYNCING`: Small circular progress ring (`16.dp`–`18.dp`).
        * `FAILED`: Compact warning/retry icon in `error`.
    * **Top-Left — Selection Checkmark:** Visible during selection mode (empty circle outline when unselected; filled
      `primary` / `secondary` circle with checkmark when selected, plus slight inward tile scale `~0.92f` with rounded
      `16.dp`–`20.dp` corner frame & tonal border, echoing `GridItem.vue`).

### 6.5 Right-Edge Timeline Scrubber

* Positioned on the right edge with `Modifier.systemGestureExclusion()` so dragging never triggers Android predictive
  back.
* Touch strip stays active along the right edge while the visual **`52.dp` circular `surfaceContainerHigh` thumb** (with
  `UnfoldMore` icon, overhanging ~33% past the right screen edge) slides in smoothly while scrolling/dragging and hides
  after `1.5s` of inactivity.
* While dragging:
    * Show **year pills** (`surfaceContainerHighest`, active year highlighted in `primary`) left of the thumb so the
      finger never obscures them, always prioritizing the newest and oldest year marks.
    * Show the floating **Montserrat month tooltip** (`"Nov 2026"`, `primaryContainer`, asymmetric
      `RoundedCornerShape(topStart = 16.dp, bottomStart = 16.dp, topEnd = 4.dp, bottomEnd = 4.dp)` matching web)
      anchored to the user's touch Y.
    * Trigger subtle haptic tick on drag start (`SegmentFrequentTick`) and when crossing **year** boundaries
      (`ContextClick`).

### 6.6 Multi-Selection Mode

* Triggered by long-press (plus drag-across-items) on any timeline tile.
* Top bar swaps cleanly to a contextual selection header (`×` close, selected count in Montserrat, `Select all`).
* Bottom exposes a floating `CircleShape` contextual action pill (Share, Add to Album, Free up space / Delete) with
  `primaryContainer` / `surfaceContainerHigh` tonal contrast.

### 6.7 Media Viewer (`MediaViewerScreen`)

* **Canvas:** `#000000` background at rest (fading backdrop opacity on swipe-down so `TimelineScreen` shows underneath),
  full edge-to-edge behind translucent system bars (forcing light status/nav bar icons with
  `isAppearanceLightStatusBars = false` and `isAppearanceLightNavigationBars = false` even in light mode, hiding
  system bars when chrome is toggled off, and restoring system bars immediately when swipe-down dismiss commits in Light
  Mode and in `DisposableEffect` on exit). While the viewer overlay is open above `NavHost`, suppress accessibility and
  focus (`hideFromAccessibility()` / non-focusable) on the underlying `NavHost` container so TalkBack and keyboard
  focus stay inside the viewer. Ensure `BackHandler` precedence layers cleanly (`DateTimeEditDialog` /
  `MediaInfoBottomSheet` > `MediaViewerScreen` > `MainScreen` Search).
* **Top Floating Bar & Overflow Menu:**
    * Compact `CircleShape` `Back` pill, title/date/counter pill (update counter/title text directly without
      `animateContentSize()` during pager drags so changing label widths never trigger spring relayouts mid-swipe), and
      top-right `⋮` overflow menu button using dark translucent scrims (`Color.Black.copy(alpha = 0.45f)`–`0.6f`).
    * **Top-right overflow menu items:** `About` (fallback trigger for the info sheet), `Copy` (photos only; hidden on
      videos and missing items), `Rotate clockwise` (JPEG/PNG/WebP photos only; hidden on videos, panoramas, missing
      items, offline `SYNCED` items, and unsupported MIME types), `Reprocess` (hidden when `missing_since != null`),
      plus video-only `Playback speed` (opens speed selector UI) and `Loop video` toggle.
* **Bottom Floating Pill:**
    * Compact `CircleShape` pill hosting **Share**, **Add to** (album), and **Bin**—no Info button in the bottom pill
      (Info is accessed via swipe-up or `About` in the top-right menu).
    * For videos, floating playback controls (seek bar driven by isolated position state-reads, play/pause, mute,
      quality selector) sit cleanly above the bottom action pill and fade together on tap. Video rendering uses
      `ContentFrame` + `rememberPresentationState(player, keepContentOnReset = isQualitySwitch)` (resetting the shutter
      on `mediaId` change until `onRenderedFirstFrame`) with `SURFACE_TYPE_TEXTURE_VIEW` so scale/clip/translation
      transforms animate cleanly and switching directly between videos always shows the incoming video's thumbnail
      shutter (`exoPlayer.stop()` + `clearMediaItems()` synchronously on page change).
* **Gestures:**
    * Swipe horizontally (`HorizontalPager(beyondViewportPageCount = 1)`) between items, staying on fast previews while
      swiping and upgrading to full-res once settled or zoomed; paging is locked (`userScrollEnabled = false`) while the
      active page is zoomed in.
    * Pinch / double-tap to zoom (`Telephoto`).
    * Tap photo/video to toggle chrome visibility smoothly (wired via `ZoomableAsyncImage`'s `onClick`).
    * **Swipe down** (single-pointer, unzoomed `(zoomableState.zoomFraction ?: 0f) <= 0.01f`, `|dy| > |dx|`) to
      interactively drag/shrink over the visible timeline and dismiss (using the idle-synced timeline position, or
      falling back to scale-to-center + fade + settle sync if dismissed mid-fling).
    * **Swipe up** (single-pointer, unzoomed `(zoomableState.zoomFraction ?: 0f) <= 0.01f`, `|dy| > |dx|`) to reveal
      the **Media Info Sheet** (`30.dp` top-rounded dark `surfaceContainerHigh` modal sheet showing capture date/time in
      Roboto/Montserrat with an edit trigger/row, timezone label—tappable on server-backed items to edit `±HH:MM`
      offset, matching web's `EditDateTimeCard.vue`—weather summary checking `temperature != null` and rendering night
      icons when `is_daytime == false`, camera/lens/exposure pills, albums list, and a location card with resolved place
      name + coordinates + `"Open in Maps"` action; embedded mini-map deferred until MapLibre is added).
    * **Date/Time & Timezone Edit Dialog (`28.dp` dialog):** Opens from the Info sheet's date/time or timezone row with
      Material 3 `DatePicker` + `TimePicker` (preserving the item's existing `:ss` seconds) and `±HH:MM` offset controls
      (tracking date/time vs. offset dirtiness independently), showing an inline saving spinner while updating and
      keeping the viewer anchored on the item even if its date moves it to another month.

---

## 7. Compose Token Reference Skeleton

```kotlin
object PhotosRadii {
    val Sheet = 30.dp
    val Dialog = 28.dp
    val Card = 24.dp          // Daily cards, Album cards, Month outer corners
    val NestedCard = 16.dp    // Concentric inner content (24.dp - 8.dp padding)
    val TileOverlay = 6.dp    // Video duration badge inside grid cell
    val GridTile = 0.dp       // Sharp inner photo grid items
    val GridGap = 2.dp
}

val PhotosShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(30.dp),
)
```