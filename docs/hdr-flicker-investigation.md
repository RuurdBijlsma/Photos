# HDR Brightness Flicker Investigation (`MediaViewerScreen` / `ZoomablePhotoPage`)

## 1. Symptom Summary

When opening a strong-HDR (`Gainmap`) photo from the timeline or swiping to it in `MediaViewerScreen`, a brief
brightness flicker occurred right as the swipe/settle ended:

- **Visual behavior:** Brightness briefly increased slightly for a few frames, then returned to the exact same
  steady-state HDR brightness.
- **Affected items:**
    - Local photos (`localContentUri != null`) with strong HDR highlights (not subtle HDR such as a slightly brighter
      sky).
    - Cached cloud photos (`initialFullResFile != null` / `full_res_cache`), **not** first-load cloud photos.
    - Reproduced on cold start as well as swiping away and swiping back.
- **Not captured in screen recording:** Screen recordings showed the low-res → high-res sharpness change, **not** the
  brightness flicker.
- **Orientation quirk:** Rotating `Portrait → Landscape → Portrait` permanently stopped the flicker for the rest of the
  app process—even after navigating back to `TimelineScreen` (which resets `window.colorMode = COLOR_MODE_DEFAULT` and
  destroys `MediaViewerViewModel`) and reopening the viewer. Restarting the app in portrait brought the flicker back.

---

## 2. Hard Evidence & Test Results

### A. Screen Recording & HDR Intensity

- **Observation:** The sharpness transition appeared in Android screen recordings, while the brightness pulse did not.
- **Conclusion:** Standard Android screen recording captures an SDR-composited `VirtualDisplay` (clamped at `1.0x` SDR
  white). The flicker therefore occurred strictly above `1.0x` SDR white (in HDR headroom / `Gainmap` rendering).

### B. Isolating the Trigger (`FULL_RES_SETTLE_DELAY_MS`)

- **Test:** Changed `FULL_RES_SETTLE_DELAY_MS` in `ZoomablePhotoPage.kt` from `180L` to `1800L`.
- **Result:** The flicker (and the `1 → 120 → 1` FPS burst) moved from ~180 ms to **1.8 seconds after settle**.
- **Conclusion:** The trigger is specifically the transition from `PreviewZoomableLayer` (Coil `ImageDecoder` `1440px`
  `Bitmap` + `Gainmap`) to `fullResImageState` (`ZoomableAsyncImage` / Telephoto `SubSamplingImage`).

### C. Isolating OS / Display Pipeline Factors

1. **Disable HW overlays (Developer Options):**
    - **Result:** Flicker persisted.
    - **Conclusion:** Not caused by SurfaceFlinger switching between HWC (`Device`) overlays and GPU (`Client`)
      composition.
2. **LTPO `1 Hz → 120 Hz → 1 Hz` refresh-rate transitions:**
    - **Observation:** FPS counter ran `120` on open → `1` while settled → `120` for a few frames at `1.8 s` → `1`.
    - **Test:** After settling at `1 Hz` (`> 2 s`), tapped the photo to hide/show viewer chrome (`180 ms` fade animation
      waking `1 Hz → 120 Hz → 1 Hz` without changing the image layer).
    - **Result:** **No flicker.**
    - **Conclusion:** The `1 Hz ↔ 120 Hz` refresh-rate jump alone does not cause the flicker.
3. **Orientation cycle (`Portrait → Landscape → Portrait → Timeline → MediaViewer`):**
    - **Result:** Flicker remained gone after returning to `TimelineScreen` and reopening `MediaViewerScreen`.
    - **Conclusion:** Because `MainActivity` has no `configChanges` (recreating the `Activity`, `Window`, and Compose
      tree on rotation) and `TimelineScreen` resets `window.colorMode` and clears `MediaViewerViewModel`, the rotation
      changes a sticky process/display-pipeline state rather than app/ViewModel state.

### D. `HDR_TRACE` Telemetry (`Display.hdrSdrRatio`, `Window.colorMode`, `Window.desiredHdrHeadroom`)

Logged `Display.registerHdrSdrRatioChangedListener`, `window.colorMode`, `window.desiredHdrHeadroom`, and
`ZoomablePhotoPage` states (`dual`, `displayed`, `fullQ`, `fullResReady`, `previewGainmap`).

- **Before rotation (flicker occurred):**
    - Initial open (`local_10676`): `our setHdrColorMode: 0 -> 2`; `hdrSdrRatio` ramped smoothly `1.0 -> 3.7963395`.
    - At `1.8 s` (`local_10676`):
        - `dual=true` (`t=741212758`)
        - `displayed=true fullQ=true fullResReady=false` (`t=741212994`, `+236 ms`)
        - `fullResReady=true` (`t=741213005`, `+11 ms`)
        - `colorMode=2`, `desiredHeadroom=0.0`, `hdrSdrRatio=3.7963395` stayed completely flat.
    - Swipe to `local_10673`:
        - `dual=true` (`t=741217765`)
        - `displayed=true fullQ=true fullResReady=false` (`t=741217964`, `+199 ms`)
        - `fullResReady=true` (`t=741217978`, `+14 ms`)
        - `colorMode=2`, `desiredHeadroom=0.0`, `hdrSdrRatio=3.7963395` stayed completely flat.
- **After `Portrait → Landscape → Portrait` (no flicker):**
    - `colorMode=2`, `desiredHeadroom=0.0`, `hdrSdrRatio=3.7963395` were identical and flat (`dual=true -> fullQ=true`
      took `45 ms` / `43 ms`, `fullResReady=true` `+8–10 ms` later).
- **Conclusion:** The flicker is **not** caused by `window.colorMode` toggling (`2 → 0 → 2`), `desiredHdrHeadroom`
  changes, or OS `Display.hdrSdrRatio` overshooting.

### E. Isolating the Preview vs. `SubSamplingImage` Layers

1. **`canUpgrade = false` (staying on `PreviewZoomableLayer`):**
    - **Result:** **No flicker.**
    - **Conclusion:** Coil `ImageDecoder` `1440px` `Bitmap` + `Gainmap` alone does not flicker.
2. **`TEST_SUBSAMPLING_ONLY = true` (`SubSamplingImage` mounted from `t = 0`, no `PreviewZoomableLayer`):**
    - **Result:** **No flicker.**
    - **Note:** On open (`local_10676`), `SubSamplingImage` mounted behind `HeroImageOverlay` (~300 ms open transition).
      On swipe (`local_10673`), it pre-mounted offscreen via `beyondViewportPageCount = 1`.
    - **Conclusion:** `SubSamplingImage` does not flicker once it has already drawn/settled before becoming visible.
3. **`TEST_SDR_PREVIEW = true` (`bypassPlatformThumbnail = false` + skipping `initialFullResFile`):**
    - **Trace:** `previewGainmap=false` on local photos. Swiping `10676 → 10673` caused `our setHdrColorMode: 2 -> 0`,
      ramping `hdrSdrRatio` down `3.7963395 -> 1.0`, then back `1.0 -> 3.7963395` at `1.8 s`.
    - **Visual:** Showed low-res SDR for ~2 s followed by a two-stage HDR load (`< 0.3 s`) without the brief brighter
      overshoot.
    - **Conclusion:**
        - Explains why first-load cloud photos (SDR `1440p.avif` preview → HDR `SubSamplingImage`) never had the
          overshoot while local and cached-cloud photos (HDR `ImageDecoder` preview → HDR `SubSamplingImage`) did.
        - Also confirms `bypassPlatformThumbnail = true` should **not** be removed, as SDR local thumbnails cause full
          `colorMode` (`2 → 0 → 2`) and `hdrSdrRatio` (`3.8 → 1.0 → 3.8`) cycling on every swipe.

### F. Pinpointing the Exact Handoff Defect (`TEST_USE_FULL_RES_ALPHA` + `TEST_DELAY_PREVIEW_UNMOUNT_MS`)

In `ZoomablePhotoPage.kt`:

- `fullResImageState` (`ZoomableAsyncImage`) is placed **underneath** `PreviewZoomableLayer` in the `Box`.
- Previously, `fullResImageState` had `Modifier.graphicsLayer { alpha = if (fullResReady) 1f else 0f }`, and
  `PreviewZoomableLayer` unmounted on the exact frame `isImageDisplayedInFullQuality` became `true`.

Tests with `TEST_SDR_PREVIEW = false` and `FULL_RES_SETTLE_DELAY_MS = 1800L`:

1. `TEST_USE_FULL_RES_ALPHA = false`, `TEST_DELAY_PREVIEW_UNMOUNT_MS = 0L`: **Flicker still present.**
2. `TEST_USE_FULL_RES_ALPHA = false`, `TEST_DELAY_PREVIEW_UNMOUNT_MS = 1500L`: **Zero flicker** (neither at `1.8 s` nor
   at `3.3 s`, verified 3× from cold start).
3. **Threshold sweep (`TEST_USE_FULL_RES_ALPHA = false`):**
    - `0 ms`: Flicker
    - `50 ms`: Flicker
    - `100 ms`: Flicker
    - `150 ms`: Tiny flicker (borderline)
    - **`200 ms`**: **No flicker**
    - `1500 ms`: **No flicker**

---

## 3. Root Cause & Fix Conclusion

### Verified Root Cause in App Code

1. `Modifier.graphicsLayer { alpha = 0f }` prevented Compose/HWUI from drawing `ZoomableAsyncImage(fullResRequest)` (
   `SubSamplingImage`) until `fullResReady = true` unmounted `PreviewZoomableLayer`.
2. Even with `alpha = 1f`, unmounting `PreviewZoomableLayer` immediately when `isImageDisplayedInFullQuality` became
   `true` exposed `SubSamplingImage` during a **~150–200 ms post-first-draw settle window** (matching the observed
   `120 fps` multi-frame burst and `50–150 ms` flicker threshold).
3. Allowing `fullResImageState` to draw at `alpha = 1f` **underneath** the opaque `PreviewZoomableLayer` (once
   `PreviewZoomableLayer` is ready) and keeping `PreviewZoomableLayer` mounted for **at least `200 ms`** (e.g. `250 ms`
   for margin) after `isImageDisplayedInFullQuality` turns `true` lets `SubSamplingImage` complete its initial
   draw/settle hidden behind `PreviewZoomableLayer`, eliminating the flicker completely.

### Working Hypothesis for Underlying Rendering / Orientation Behavior

- *Hypothesis (not directly instrumented inside Telephoto/Skia internals):* During the first ~150–200 ms after
  `SubSamplingImage` begins drawing (`alpha = 1f`), Telephoto's tile rendering / `BaseTile` + foreground `Gainmap` tile
  compositing in HWUI/Skia runs across multiple frames (`120 fps` burst) and triggers a brief HDR luminance overshoot in
  the pre-rotation compositor/display pipeline. `Portrait → Landscape → Portrait` resets that device compositor/display
  state, while keeping `PreviewZoomableLayer` on top during those `200–250 ms` hides the unstable frames entirely from
  the compositor/screen in all orientations.

---

## 4. Recommended Clean Fix (`ZoomablePhotoPage.kt`)

- Remove temporary `HDR_TRACE` logging and `TEST_*` scaffolding from `ViewerSystemBars.kt`, `ZoomablePhotoRequests.kt`,
  and `ZoomablePhotoPage.kt` (including restoring `FULL_RES_SETTLE_DELAY_MS = 180L`).
- Keep `fullResImageState` drawn at `alpha = 1f` underneath `PreviewZoomableLayer` once `previewReady` is true (using
  `alpha = if (fullResReady || previewReady) 1f else 0f` so `fullResImageState` never flashes uncovered if
  `PreviewZoomableLayer` hasn't painted yet).
- Wait `~250 ms` (above the `200 ms` observed threshold) after `showDualOverlay && isFullResFullQuality` before setting
  `hasDisplayedFullResInFullQuality = true` and unmounting `PreviewZoomableLayer` (ensuring early-zoom in-place upgrades
  via `isPreviewZoomed` remain immediate).