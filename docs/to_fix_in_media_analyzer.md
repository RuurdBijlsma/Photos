1. **The offset you type is thrown away on any photo with GPS** (`common_services/src/api/photos/service.rs:657-677`,
   `media_analyzer-0.10.6/src/time/logic.rs:78-119`)
    - **What this is:** After writing `OffsetTimeOriginal`, the item is re-analyzed and the DB's
      `timezone_offset_seconds` is overwritten from the analysis result.
    - **Problem:** The analyzer prefers a timezone derived from GPS coordinates (priority 2) over an explicit offset
      tag (priority 3). So for a photo with GPS, the offset you just wrote is ignored: set `+09:00` on a photo taken in
      Greece and the DB comes back with `+03:00` / `Europe/Athens`. All three test photos have GPS, and the two Greek
      ones resolve to `+03:00` for June 2024, so the new test only passes for whichever row it happens to pick (the
      French one, `+02:00`). It is a coin flip, not a passing test.
    - **Fix:** Make an explicit offset outrank GPS in `media_analyzer` (`apply_priority_logic`: move the explicit-offset
      branch above the GPS branch, bump the version here). That is the only fix that keeps file and DB agreeing. Quick
      stopgap if you want it now: for an offset-only edit, skip the re-analysis and update `timezone_offset_seconds` in
      the DB directly.
    - **If we skip it:** The offset editor silently does nothing for most real photos (phone photos have GPS), and the
      integration test is flaky.

2. **Offset edits never persist for videos** (`common_services/src/utils.rs:53-68`)
    - **What this is:** For videos you write `CreateDate`/`ModifyDate` (no zone) plus `CreationDate` with the offset.
    - **Problem:** The analyzer reads video `CreateDate` as **UTC** and only takes video local time from
      `Time:DateTimeOriginal`, which MP4s do not have. `Keys:CreationDate` is never read, and `OffsetTime*` is not
      written for videos. So after the re-analysis, `timezone_offset_seconds` and `timezone_name` come back `NULL`. Set
      an offset on `VID_20190223_155900.mp4` and the card goes back to "Listed in the local timezone". `taken_at_utc`
      also becomes the local wall time read as UTC.
    - **Fix:** Either teach `media_analyzer` to use `Keys:CreationDate`'s offset for videos, or treat video offset edits
      as unsupported (hide/disable the offset editor for videos) instead of pretending they work.
    - **If we skip it:** Every video offset edit is silently lost, and nothing in the suite notices.

---

both should be fixed in media_analyzer

---

## Done in media_analyzer 0.10.7

1. `apply_priority_logic` (`src/time/logic.rs`) now tries "naive local time + explicit offset tag" before the GPS-derived timezone, so
   `OffsetTimeOriginal` wins over GPS. When the GPS zone happens to have the same offset at that instant, its IANA name is kept
   for display (`"Europe/Athens"` instead of `"+03:00"`); otherwise `timezone.name` becomes the offset string.
2. `extract_time_components` (`src/time/extraction.rs`) reads `Keys:CreationDate` (family 2 group `Time`, so `Time:CreationDate`) for
   videos. It supplies both the video's local wall time and its offset, which then feeds the same explicit-offset priority. Videos
   without that tag behave exactly as before (`CreateDate` read as UTC).

Verified with the full `test_integration` suite against the local crate (via a temporary `[patch.crates-io]`): both
`test_update_datetime_edits_exif` and `test_update_video_datetime_edits_exif` pass.

Still needed before merging: publish 0.10.7, then `cargo update -p media_analyzer` in `server/` (the manifests now require
`0.10.7`, the lock is still on 0.10.6).
