# Takeout Streaming Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import Google Photos Takeout ZIPs by reading them in place, so a 148 GB batch needs no extra app-data space, and make `POST /api/upload/takeout/start` return immediately.

**Architecture:** The worker indexes every ZIP's central directory and small JSON entries (album `metadata.json`, media sidecars) without decompressing any media, creates/reuses albums, dedupes media filenames against the database, then streams only the new media entries straight out of the ZIPs into `<media_root>/<user>/takeout/`. ZIPs stay on the app-data volume and are deleted only after a fully successful batch. The HTTP start route stops repairing uploads; the worker repairs each ZIP before reading it.

**Tech Stack:** Rust 2024 (worker crate, axum API), `zip` 9.2.2, sqlx/Postgres, nginx (web image).

**Spec:** `docs/google-photos-takeout-investigation.md` plus the design decisions in this document.

## Design decisions (the behavior this plan implements)

- Never extract a Takeout ZIP into `app_data`. The batch uses `cache_root/takeout_tmp` today and that path is removed entirely.
- Media dedupe keeps today's semantics: a media item is "already imported" when `media_item.filename` (the sanitized name) exists for the user. Existing items are linked to the Takeout albums, not copied.
- Albums keep today's semantics: one album per top-level folder under `Takeout/Google Photos/` (or `Google Photos/`), title/description from that folder's `metadata.json`, merged by title.
- Sidecar timestamps keep today's semantics, including a sidecar that lives in a different archive than its media, and the `fallback_taken_at` payload sent to `IngestMetadata`.
- One unreadable/corrupt ZIP entry is skipped: log a warning with the ZIP id, entry name and reason, delete its partial file, keep going, and report it in the summary. A whole archive that cannot be opened or repaired still fails the job.
- A destination file in `<user>/takeout/` that does not have a database row is a leftover from an earlier failed attempt and is replaced. No `_1` suffixed duplicates.
- ZIPs and their `.info` files are deleted only after albums, copies, links and ingest jobs are all done. On failure they stay, and the job's `last_error` is the explanation.
- `/api/upload/takeout/start` no longer repairs uploads; it only reports ready ZIPs and enqueues the job. The worker calls `repair_tus_upload` per ZIP before reading it, which already exists.
- No new job types, no migration, no per-job progress UI. Visibility comes from log lines and the existing job row.
- Only `Takeout/Google Photos/<Album>/<file>` and `Google Photos/<Album>/<file>` are recognized. The old importer's last-resort "treat every top-level folder as an album" fallback is dropped on purpose; if the index finds no media at all, the job fails loudly instead of silently importing nothing.
- The copy destination is always `<user>/takeout/<sanitized basename>`; ZIP paths are never recreated on disk, so the entry-name validator plus the sanitized basename is the whole path-safety story. Do **not** use `ZipFile::enclosed_name()`: zip 9.2.2 rejects any component ending in a space on Windows, and real Takeout folders are named `Photos from 2023 ` / `Vacation 2023 `, so every entry is silently skipped there (this already breaks the integration fixture on Windows). Use `name()` and reject absolute paths, drive prefixes, `.` and `..`, then trim each component. `.part` files can live in the destination folder because the watcher ignores managed folders (`takeout` is in `MANAGED_FOLDERS`), so no separate scratch directory is needed.
- When a sanitized filename appears more than once, the first occurrence wins: ZIP order (`payload.tus_upload_ids`, already sorted by the API) and then central-directory order. Album memberships are unioned.
- Out of scope: FileLoft's own peak of parts + assembled file per in-flight upload during finalization.

## Global Constraints

- Rust edition 2024, rustfmt `max_width = 100`, clippy `pedantic` + `nursery` are errors.
- `#![deny(clippy::unwrap_used)]` is active in production crates: no `unwrap()`. `expect("...")` is fine in tests.
- Blocking work (ZIP reads and file copies) runs inside `tokio::task::spawn_blocking`. Do not add synchronous file I/O to an `async fn` that runs on the runtime.
- No new dependencies: use the existing `zip`, `sanitize-filename`, `chrono`, `tracing` dependencies of the `worker` crate.
- Do not change `common_services::api::upload::service::repair_tus_upload`; it is still used by the worker and by its own tests.
- No new SQL, so `server/.sqlx` does not change. If a query is added anyway, run `./scripts/update_sqlx.ps1` before the PR.
- Branch from `develop`; commit messages use conventional prefixes.

## Review Focus

- An archive without the `Takeout/` wrapper (`Google Photos/...`) must import identically — pinned by `takeout_parts_handles_roots_and_trailing_spaces` (Task 3).
- A sidecar stored in a different archive than its media must still supply the date — pinned by the existing `test_takeout_import_flow` fixture (`takeout_new.png` sidecar lives in ZIP 2, media in ZIP 1) (Tasks 4-5).
- A ZIP entry with a bad CRC must be skipped while the rest of the batch imports — pinned by `skips_corrupt_entry_and_continues` (Task 5).
- A destination left by an earlier failed attempt must be replaced, not duplicated as `name_1.ext` — pinned by `copies_over_an_existing_destination` (Task 3).
- A ZIP entry name can never escape the destination — pinned by `takeout_parts_rejects_unsafe_names` (Task 3).
- The import must not create anything under `cache_root` — pinned by the existing job-scoped `takeout_tmp` assertion in `assert_cleanup` (Task 4).

## Compared with `docs/google-photos-takeout-plan.md`

That document is design-level; this one is the task breakdown. Its design agrees on streaming, worker-side repair, and leaving the ingest pipeline alone. Ideas taken from it:

- Per-ZIP progress lines and a batch summary with counts and bytes (Task 3/4).
- A loud failure when the index finds no media at all, instead of repeating the silent no-op from production (Task 4).
- An explicit deterministic winner for duplicate filenames (design decisions).
- A statement of the path-safety invariant and why `.part` files are safe inside `takeout/` (design decisions).

Deliberately not taken, to keep this a one-off-sized change:

- Per-item checkpoints / resumed batches. The agreed behavior is retry from scratch; stable destination names plus overwrite make a retry correct.
- A separate scratch directory on the media volume for temporary files. The watcher already ignores `takeout/` (`MANAGED_FOLDERS`), so `.part` files next to the destination are invisible and need no cleanup machinery beyond removing the part on error.
- A ZIP-size preflight against the TUS `.info`. A truncated archive already fails in `ZipArchive::new` with a readable error, and `repair_tus_upload` runs first.
- Content-hash deduplication. Filename dedupe is the current contract and what the user asked for.
- The old root fallback that treats any top-level folder as an album. Standard Takeout layouts are supported; anything else fails the empty-index guard instead of guessing.

---

## File Structure

| File | Responsibility |
|---|---|
| `web/nginx.conf` | Give upload requests a proxy read timeout longer than TUS finalization. |
| `server/crates/apps/api/src/routes/upload/handlers.rs` | `/upload/takeout/start` reports ready ZIPs and enqueues; no repair, more logging. |
| `server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs` | Batch orchestration: repair, index, albums, dedupe, copy batches, link, enqueue, cleanup. |
| `server/crates/apps/worker/src/handlers/ingest_takeout_batch/archive.rs` | ZIP-only logic: path normalization, index pass, copy pass, sidecar lookup. Unit-tested without a database. |
| `server/crates/test_integration/src/tests/test_takeout.rs` | End-to-end expectations, including the corrupt-entry skip. |

---

### Task 1: nginx upload timeouts

**Files:**
- Modify: `web/nginx.conf:59-74`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing code-facing.

- [ ] **Step 1: Add the timeouts to the API location**

In the `location ~ ^/(api)` block, after `proxy_buffering off;`, add:

```nginx
        # Large TUS PATCH requests finalize (concatenate parts) before answering; allow well over
        # nginx's 60s default so a slow disk does not cut the upload mid-finalize.
        proxy_read_timeout 600s;
        proxy_send_timeout 600s;
```

- [ ] **Step 2: Verify the config parses**

Run from the repo root (Docker or Podman both accept the mount form):

```powershell
docker run --rm --mount "type=bind,source=$((Get-Location).Path)\web\nginx.conf,target=/etc/nginx/conf.d/default.conf,readonly" nginx:alpine-slim nginx -t
```

Expected: `nginx: configuration file /etc/nginx/nginx.conf test is successful`.

- [ ] **Step 3: Commit**

```powershell
git add web/nginx.conf
git commit -m "fix: allow long uploads past the web proxy timeout"
```

---

### Task 2: `/upload/takeout/start` stops repairing uploads

**Files:**
- Modify: `server/crates/apps/api/src/routes/upload/handlers.rs:18` (imports), `:108-181` (`pending_takeout_upload_ids`)
- Test: `server/crates/test_integration/src/tests/test_takeout.rs:521-542`

**Interfaces:**
- Consumes: `common_services::api::upload::service::repair_tus_upload` exists but is deliberately no longer called here.
- Produces: `pending_takeout_upload_ids(&ApiContext, i32) -> Result<Vec<String>, AppError>` keeps its signature; it now returns IDs without touching their files.

- [ ] **Step 1: Establish the baseline**

Run the integration suite before touching anything (same command as Task 4 Step 5). Expected: PASS. This task changes request latency, not import semantics: request-side repair was harmless in the test because the ZIPs are tiny, so there is no honest red test to write here. The end-to-end suite plus the production log check are the verification.

- [ ] **Step 2: Update the integration test to stop expecting a repair during `/start`**

In `test_takeout.rs`, after `let (started, zip_count) = start_takeout_import(client, api_url).await?;` (the second call, around line 533), delete these three assertions:

```rust
    assert_eq!(
        std::fs::read(tus_dir.join(&zip1_id))?,
        zip1_bytes,
        "the interrupted finalization should be repaired before enqueueing"
    );
    assert!(!tus_dir.join(format!("{zip1_id}_part_0")).exists());
    assert!(!tus_dir.join(format!("{zip1_id}_part_1")).exists());
```

Keep `assert!(started);` and `assert_eq!(zip_count, 2);`. The ZIP stays truncated until the worker repairs it; the existing `wait_for_takeout_job` and `assert_cleanup` calls now prove the worker repaired and consumed it. The `tus_dir` and `zip1_bytes` locals stay in use by `simulate_interrupted_finalize`, so no unused-variable cleanup is needed.

- [ ] **Step 3: Remove the repair from `pending_takeout_upload_ids`**

Replace the per-upload block:

```rust
        let id = info.id.to_string();
        if claimed.contains(&id) {
            continue;
        }
        match repair_tus_upload(&tus_dir, &id).await {
            Ok(_) => upload_ids.push(id),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                return Err(AppError::Conflict(format!(
                    "Takeout upload {id} is incomplete: {e}. Wait for it to finish or reselect the ZIPs."
                )));
            }
            Err(e) => return Err(AppError::Internal(e.into())),
        }
```

with:

```rust
        let id = info.id.to_string();
        if claimed.contains(&id) {
            claimed_count += 1;
            continue;
        }
        upload_ids.push(id);
```

Add two counters at the top of the function (`let mut incomplete_count = 0usize; let mut claimed_count = 0usize;`), increment `incomplete_count` in the branch that currently `continue`s when `info.size != Some(info.offset)`, and after `upload_ids.sort_unstable(); upload_ids.dedup();` log:

```rust
    info!(
        "Takeout import start for user {user_id}: {} ZIP(s) ready, {claimed_count} claimed by an active import, {incomplete_count} still uploading",
        upload_ids.len()
    );
```

Import `tracing::info` next to the existing `error`; drop `warn` from that import (it is unused) and remove `repair_tus_upload` from the `common_services::api::upload::service` import, leaving only `decode_upload_token`.

- [ ] **Step 4: Run the integration test again**

Same command as Step 1.

Expected: PASS. The worker repairs the truncated ZIP before indexing it (Task 4 keeps that call).

- [ ] **Step 5: Lint**

```powershell
cargo fmt --all
cargo clippy --no-deps --all-features --tests -- -D clippy::all -D clippy::pedantic -D clippy::nursery
```

Expected: no warnings.

- [ ] **Step 6: Commit**

```powershell
git add server/crates/apps/api/src/routes/upload/handlers.rs server/crates/test_integration/src/tests/test_takeout.rs
git commit -m "fix: return from takeout start without repairing uploads"
```

---

### Task 3: ZIP index and copy module

**Files:**
- Create: `server/crates/apps/worker/src/handlers/ingest_takeout_batch/archive.rs`
- Modify: `server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs:1-27` (add `mod archive;`)

**Interfaces:**
- Consumes: `sanitize_filename::sanitize(&str) -> String`, `chrono::DateTime<Utc>`, and a caller-supplied `is_media: impl Fn(&Path) -> bool` predicate (so the module needs no settings and stays unit-testable).
- Produces (used by Task 4):

```rust
pub(super) struct EntryRef { pub upload_id: String, pub index: usize }

pub(super) struct MediaEntry {
    pub entry: EntryRef,
    pub folder: String,       // trimmed album folder name inside `Google Photos`
    pub filename: String,     // sanitized name, also the on-disk name
    pub size: u64,
    pub albums: HashSet<String>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct AlbumMetadata { pub title: Option<String>, pub description: Option<String> }

#[derive(Debug, Default)]
pub(super) struct TakeoutIndex {
    pub albums: BTreeMap<String, AlbumMetadata>,
    pub media: HashMap<String, MediaEntry>,                 // sanitized filename -> first occurrence
    pub sidecars: HashMap<(String, String), Option<DateTime<Utc>>>, // (folder, lowercased name)
}

impl TakeoutIndex { pub fn merge(&mut self, other: Self); }

pub(super) fn index_archive(
    index: &mut TakeoutIndex,
    upload_id: &str,
    zip_path: &Path,
    is_media: impl Fn(&Path) -> bool,
) -> Result<()>;

pub(super) struct CopyRequest {
    pub entry_index: usize,
    pub filename: String,
    pub size: u64,
    pub fallback_taken_at: Option<DateTime<Utc>>,
}

pub(super) enum CopyOutcome {
    Copied { filename: String, size: u64, fallback_taken_at: Option<DateTime<Utc>> },
    Skipped { filename: String, reason: String },
}

pub(super) fn copy_entries(
    zip_path: &Path,
    requests: &[CopyRequest],
    dest_folder: &Path,
) -> Result<Vec<CopyOutcome>>;

pub(super) fn sidecar_timestamp(
    index: &TakeoutIndex,
    folder: &str,
    filename: &str,
) -> Option<DateTime<Utc>>;
```

- [ ] **Step 1: Declare the module and write the failing unit tests**

At the top of `ingest_takeout_batch.rs`, add `mod archive;` above the `use` statements. Create `ingest_takeout_batch/archive.rs` with the types above (bodies `todo!()` / stubs as needed) and this test module at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    const SIDECAR_TS: i64 = 1_690_000_000;

    fn build_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(*name, SimpleFileOptions::default())
                .expect("start entry");
            zip.write_all(content).expect("write entry");
        }
        zip.finish().expect("finish zip");
    }

    fn build_stored_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, content) in entries {
            zip.start_file(*name, options).expect("start entry");
            zip.write_all(content).expect("write entry");
        }
        zip.finish().expect("finish zip");
    }

    fn is_media(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "jpg" | "jpeg" | "png" | "mp4"))
    }

    #[test]
    fn takeout_parts_handles_roots_and_trailing_spaces() {
        assert_eq!(
            takeout_parts("Takeout/Google Photos/Vacation 2023 /a.jpg"),
            Some(("Vacation 2023".to_owned(), "a.jpg".to_owned()))
        );
        assert_eq!(
            takeout_parts("Google Photos/Photos from 2013/b.png"),
            Some(("Photos from 2013".to_owned(), "b.png".to_owned()))
        );
        assert_eq!(takeout_parts("Takeout/Other/a.jpg"), None);
        assert_eq!(takeout_parts("Takeout/Google Photos/a.jpg"), None);
        assert_eq!(takeout_parts("Takeout/Google Photos/Album/sub/a.jpg"), None);
    }

    #[test]
    fn takeout_parts_rejects_unsafe_names() {
        assert_eq!(takeout_parts("Takeout/Google Photos/../a.jpg"), None);
        assert_eq!(takeout_parts("/Takeout/Google Photos/Album/a.jpg"), None);
        assert_eq!(takeout_parts("C:/Google Photos/Album/a.jpg"), None);
        assert_eq!(takeout_parts("Takeout/Google Photos/Alb:um/a.jpg"), None);
    }

    #[test]
    fn indexes_albums_media_and_sidecars_without_extracting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("takeout.zip");
        let sidecar = format!(
            r#"{{"photoTakenTime": {{"timestamp": "{SIDECAR_TS}"}}}}"#
        );
        build_zip(
            &zip_path,
            &[
                (
                    "Takeout/Google Photos/Vacation 2023 /metadata.json",
                    br#"{"title": "Summer Vacation", "description": "Summer holiday"}"#,
                ),
                ("Takeout/Google Photos/Vacation 2023 /photo.jpg", b"jpeg-bytes"),
                (
                    "Takeout/Google Photos/Vacation 2023 /photo.jpg.supplemental-metadata.json",
                    sidecar.as_bytes(),
                ),
                ("Takeout/Google Photos/Photos from 2023/photo.jpg", b"jpeg-bytes"),
                ("Takeout/Google Photos/Photos from 2023/notes.txt", b"ignored"),
            ],
        );

        let mut index = TakeoutIndex::default();
        index_archive(&mut index, "upload-1", &zip_path, is_media).expect("index");

        refute_zip_extraction(dir.path());
        assert_eq!(index.albums.len(), 2);
        assert_eq!(
            index.albums["Vacation 2023"].title.as_deref(),
            Some("Summer Vacation")
        );
        assert_eq!(
            index.albums["Vacation 2023"].description.as_deref(),
            Some("Summer holiday")
        );
        let media = &index.media["photo.jpg"];
        assert_eq!(media.folder, "Vacation 2023");
        assert_eq!(media.entry.index, 1);
        assert_eq!(media.size, 10);
        assert!(media.albums.contains("Vacation 2023"));
        assert!(media.albums.contains("Photos from 2023"));
        assert_eq!(
            sidecar_timestamp(&index, "Vacation 2023", "photo.jpg"),
            Some(DateTime::from_timestamp(SIDECAR_TS, 0).expect("timestamp"))
        );
    }

    #[test]
    fn copies_over_an_existing_destination() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("takeout.zip");
        let dest = dir.path().join("takeout");
        std::fs::create_dir_all(&dest).expect("dest");
        std::fs::write(dest.join("photo.jpg"), b"leftover").expect("leftover");

        build_zip(&zip_path, &[("Takeout/Google Photos/Album/photo.jpg", b"fresh")]);
        let outcomes = copy_entries(
            &zip_path,
            &[CopyRequest {
                entry_index: 0,
                filename: "photo.jpg".to_owned(),
                size: 5,
                fallback_taken_at: None,
            }],
            &dest,
        )
        .expect("copy");

        assert!(matches!(outcomes.as_slice(), [CopyOutcome::Copied { .. }]));
        assert_eq!(std::fs::read(dest.join("photo.jpg")).expect("read"), b"fresh");
        assert_no_part_files(&dest);
    }

    #[test]
    fn skips_corrupt_entry_and_continues() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("takeout.zip");
        let dest = dir.path().join("takeout");
        std::fs::create_dir_all(&dest).expect("dest");

        let payload = b"unique-corrupt-payload-0123456789";
        build_stored_zip(
            &zip_path,
            &[
                ("Takeout/Google Photos/Album/corrupt.jpg", payload),
                ("Takeout/Google Photos/Album/good.jpg", b"good"),
            ],
        );
        // Flip a byte inside the stored payload so the CRC check fails at the end of the read.
        let mut bytes = std::fs::read(&zip_path).expect("read zip");
        let start = bytes
            .windows(payload.len())
            .position(|window| window == payload)
            .expect("payload");
        bytes[start + payload.len() / 2] ^= 0xFF;
        std::fs::write(&zip_path, bytes).expect("write zip");

        let outcomes = copy_entries(
            &zip_path,
            &[
                CopyRequest {
                    entry_index: 0,
                    filename: "corrupt.jpg".to_owned(),
                    size: payload.len() as u64,
                    fallback_taken_at: None,
                },
                CopyRequest {
                    entry_index: 1,
                    filename: "good.jpg".to_owned(),
                    size: 4,
                    fallback_taken_at: None,
                },
            ],
            &dest,
        )
        .expect("copy");

        assert!(matches!(
            &outcomes[0],
            CopyOutcome::Skipped { filename, .. } if filename == "corrupt.jpg"
        ));
        assert!(matches!(
            &outcomes[1],
            CopyOutcome::Copied { filename, .. } if filename == "good.jpg"
        ));
        assert!(!dest.join("corrupt.jpg").exists());
        assert_eq!(std::fs::read(dest.join("good.jpg")).expect("read"), b"good");
        assert_no_part_files(&dest);
    }

    fn refute_zip_extraction(dir: &Path) {
        assert_eq!(
            std::fs::read_dir(dir)
                .expect("read dir")
                .filter(|entry| entry.as_ref().expect("entry").file_type().expect("type").is_dir())
                .count(),
            0,
            "indexing must not create directories or extract media"
        );
    }

    fn assert_no_part_files(dest: &Path) {
        assert!(
            !std::fs::read_dir(dest)
                .expect("read dir")
                .any(|entry| entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".part")),
            "a failed or finished copy must not leave .part files"
        );
    }
}
```

Every test in this module is self-contained: no settings, no database, no model downloads.

- [ ] **Step 2: Run the tests to verify they fail**

```powershell
cd server
$env:ORT_DYLIB_PATH = "C:/Apps/onnxruntime/lib/onnxruntime.dll"
$env:DATABASE_URL = "postgresql://photos_user:dev-password@localhost:5432/photos"
cargo test --profile release --features load-dynamic -p worker --lib ingest_takeout_batch::archive -- --nocapture
```

Expected: compile errors for the missing functions, or `todo!()` panics once the stubs exist.

- [ ] **Step 3: Implement path normalization**

`fn safe_components(name: &str) -> Option<Vec<String>>` — reject a name starting with `/` or `\`, reject any component containing `:` (drive or alternate-data-stream prefix), and reject `.` and `..` components; split on `/` and `\`, trim each component, and drop empties. This replaces `enclosed_name()`, which silently rejects trailing-space folders on Windows.

`fn takeout_parts(name: &str) -> Option<(String, String)>` — `safe_components(name)?`, strip a leading `Takeout`, then require exactly `["Google Photos", album, file]`. Return `(album, file)`. Any other depth or root returns `None`.

- [ ] **Step 4: Implement the JSON helpers**

Move the existing `SidecarTimestamp`/`SidecarJson` structs and `parse_sidecar_timestamp(&Path)` into `archive.rs` as `parse_sidecar_timestamp(&str) -> Option<DateTime<Utc>>` (same `photoTakenTime` then `creationTime` fallback, same epoch-seconds parse). Move `split_duplicate_counter`, `sidecar_candidates` and their existing unit tests from the old handler unchanged.

`AlbumMetadata` derives `Debug, Default, Deserialize, Clone` with `title`/`description`.

`fn read_small_json(entry: &mut impl std::io::Read) -> Option<String>` — `take(1 << 20)` then `read_to_string`, returning `None` on I/O or UTF-8 error, so one unreadable sidecar just loses its timestamp.

- [ ] **Step 5: Implement `index_archive`**

Open the ZIP with `ZipArchive::new(BufReader::new(File::open(zip_path)?))?`. For each index in `0..archive.len()`:

- `archive.by_index(i)`; on `Err`, `tracing::warn!("Skipping unreadable Takeout entry {i} in {upload_id}: {e}")` and continue.
- Skip directories and entries whose `name()` fails `safe_components`.
- `takeout_parts`; skip entries outside the photos root.
- Register the folder in `index.albums` with `or_default()`.
- If the file has a `.json` extension: read it with `read_small_json`; when the file name (case-insensitive) is `metadata.json`, parse it as `AlbumMetadata` and copy `title`/`description` into the folder's metadata when present; otherwise parse the timestamp and insert `(folder, sanitize(file_name).to_lowercase()) -> Option<DateTime<Utc>>` with `or_insert` so the first occurrence wins.
- Else if `is_media(Path::new(&file_name))`: insert into `index.media` keyed by `sanitize(&file_name)` only when absent; either way insert `folder` into that entry's `albums` and set `size` from `entry.size()`.

Before returning, log one line per archive so a long index pass is visible:

```rust
info!(
    "Indexed ZIP {upload_id}: {albums} album(s), {media} media entry(ies), {sidecars} sidecar(s)"
);
```

with local counters for this archive. Do not call `zip::ZipReadOptions` anywhere; `by_index` already gives a `Read`.

- [ ] **Step 6: Implement `TakeoutIndex::merge`**

- `albums`: `entry(folder).or_default()`, then fill `title`/`description` only when the current value is `None` and the incoming one is `Some`.
- `media`: if the key exists, extend its `albums` set; otherwise insert the entry as-is (first occurrence wins).
- `sidecars`: `entry(key).or_insert(value)`.

- [ ] **Step 7: Implement `copy_entries` and `sidecar_timestamp`**

`copy_entries`: open the archive once. For every request, write `dest_folder/<filename>.part` by `std::io::copy` from `archive.by_index(request.entry_index)`, then `sync_all`. On a read/write error or a copied length different from `request.size`, remove the `.part` and push `CopyOutcome::Skipped { reason }`. Otherwise remove the destination if it exists (ignore `NotFound`; any other error is a skip), rename `.part` onto it (this is the Windows-safe replace), and push `CopyOutcome::Copied { filename, size, fallback_taken_at }` with the request's size. Opening the archive itself failing returns `Err` so the job fails.

`sidecar_timestamp`: iterate `sidecar_candidates(filename)`; the first candidate present in `index.sidecars` for this folder decides the result, returning its `Option<DateTime<Utc>>` even when it is `None` — matching today's "first matching sidecar wins, even if unparseable".

- [ ] **Step 8: Run the unit tests to verify they pass**

Same command as Step 2. Expected: 7 tests pass (the 2 moved sidecar tests plus the 5 new ones).

- [ ] **Step 9: Lint**

```powershell
cargo fmt --all
cargo clippy --no-deps --all-features --tests -- -D clippy::all -D clippy::pedantic -D clippy::nursery
```

Expected: no warnings. If items are reported as unused, leave them: Task 4 wires them in.

- [ ] **Step 10: Commit**

```powershell
git add server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs server/crates/apps/worker/src/handlers/ingest_takeout_batch/archive.rs
git commit -m "feat: index and stream takeout zip entries without extracting"
```

---

### Task 4: Rewire the batch handler to stream

**Files:**
- Modify: `server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs` (whole file)
- Test: `server/crates/test_integration/src/tests/test_takeout.rs:442-465` (`assert_cleanup` message only)

**Interfaces:**
- Consumes: everything Task 3 produces; `repair_tus_upload(&tus_dir, &upload_id)`; `AlbumStore::{find_by_name, create, upsert_collaborator, add_media_items}`; `MediaItemStore::find_existing_filenames_map`; `bulk_enqueue_full_ingest`; `app_state::MakeRelativePath`.
- Produces: `pub async fn handle(&WorkerContext, &Job) -> Result<JobResult>` keeps its signature. No public API changes.

- [ ] **Step 1: Keep the no-staging assertion in `assert_cleanup`**

`assert_cleanup` already asserts that `cache_root/takeout_tmp/<job_id>` does not exist (test_takeout.rs:455-464). Keep it exactly as it is — it is the regression check for "never stage under app data", and it is job-id scoped so leftover folders from other local runs cannot make it flaky. Only update its message to `"the import must not stage anything under app data"`.

Do not change the ZIP / `.info` deletion assertions above it.

- [ ] **Step 2: Run the suite after the rewrite**

There is no red test for this step before the rewrite (the old code deletes its staging folder, so the assertion passes either way). Run the suite once at the end of this task to prove the rewrite did not regress it.

- [ ] **Step 3: Replace `handle` with the streaming flow**

Delete `extract_zip_archive`, `extract_archives`, `locate_photos_root`, `index_files`, `import_media_item`, and the old `SidecarIndex`/`MediaIndex`/`AlbumMetadataJson`/`SidecarJson`/`SidecarTimestamp`/`split_duplicate_counter`/`sidecar_candidates`/`find_sidecar`/`parse_sidecar_timestamp` definitions (all moved or replaced by `archive.rs`). Keep `require_media_mounted`, the user/payload resolution, `create_albums` (adapted), `cleanup` (minus the extraction folder) and the `handle` entry point.

`handle` becomes:

1. Resolve `user_id`, `user_media_folder` and `payload` exactly as today.
2. `let tus_dir = settings.ingest.app_data_root.join(TUS_UPLOADS_FOLDER);`
3. For each upload id: `repair_tus_upload(&tus_dir, upload_id).await?`; then bail with `"Takeout ZIP part {path:?} is missing"` if the file does not exist.
4. `let mut index = archive::TakeoutIndex::default();` For each upload id, clone `upload_id`, `zip_path` and `settings.ingest`, then run in `tokio::task::spawn_blocking`:

```rust
let mut partial = archive::TakeoutIndex::default();
archive::index_archive(&mut partial, &upload_id, &zip_path, |path| {
    ingest.is_media_file(path)
})?;
Ok::<_, color_eyre::Report>(partial)
```

and `index.merge(partial)`. Log the album and media counts. If `index.media` is empty, bail with `"no media entries found in {n} Takeout ZIP(s); expected Takeout/Google Photos/<album>/<file>"` so a wrong archive layout fails visibly instead of silently importing nothing.
5. `let folder_to_album_id = create_albums(context, user_id, &index.albums).await?;` — iterate the map instead of `read_dir`, use the metadata for title/description, and keep the existing find-or-create/`upsert_collaborator` logic and `AlbumSort::DateAsc`. Return `HashMap<String, String>` (folder -> album id).
6. `let all_filenames: Vec<String> = index.media.keys().cloned().collect();` then `MediaItemStore::find_existing_filenames_map(&context.pool, user_id, &all_filenames).await?`.
7. Create `user_takeout_folder = media_root/<user_media_folder>/<TAKEOUT_IMPORT_FOLDER>`.
8. Partition in one loop over `index.media`:
   - Existing filename: apply the same album-linking logic as today (`existing_album_links: BTreeMap<String, Vec<String>>`, album id -> media ids).
   - New filename: resolve `album_ids` from `media.albums` through `folder_to_album_id`, push a `CopyRequest` into `requests_by_upload: BTreeMap<String, Vec<CopyRequest>>` keyed by `media.entry.upload_id`, and remember `album_ids` per filename in `album_ids_by_filename: HashMap<String, Vec<String>>`.
9. For each `(upload_id, requests)` in `requests_by_upload`, run `archive::copy_entries(&tus_dir.join(&upload_id), &requests, &user_takeout_folder)` in `spawn_blocking`. For `Copied { filename, size, fallback_taken_at }`: add `size` to a running `copied_bytes`, compute `relative_path` with `make_relative(&settings.ingest.media_root)?`, and push `(relative_path, Some(IngestMetadataPayload { album_ids, remote_user_identity: None, fallback_taken_at }))`. For `Skipped { filename, reason }`: `warn!("Skipping {filename} from {upload_id}: {reason}")` and collect `(upload_id, filename)` in a `skipped` vector. After each ZIP batch, log `"ZIP {upload_id}: copied {copied}/{total} entries, {skipped_here} skipped, {bytes} bytes"` so a 40-minute import shows progress.
10. Link existing media to albums exactly as today, then `bulk_enqueue_full_ingest(...)` for the new media.
11. Log the summary:

```rust
    info!(
        "✅ Takeout batch finished: {} existing media linked to albums, {} new media enqueued for ingest ({} bytes copied), {} skipped",
        linked_existing_count, new_count, copied_bytes, skipped.len()
    );
    if !skipped.is_empty() {
        let shown: Vec<String> = skipped
            .iter()
            .take(50)
            .map(|(zip, file)| format!("{zip}/{file}"))
            .collect();
        warn!(
            "⚠️ Skipped {} unreadable Takeout item(s) (first {}): {}",
            skipped.len(),
            shown.len(),
            shown.join(", ")
        );
    }
```

12. `cleanup(context, &payload).await` (drop the `extract_dir` argument) and `Ok(JobResult::Done)`.

- [ ] **Step 4: Run the full gate on the worker crate**

```powershell
cargo fmt --all
cargo clippy --no-deps --all-features --tests -- -D clippy::all -D clippy::pedantic -D clippy::nursery
cargo test --profile release --features load-dynamic -p worker --lib ingest_takeout_batch -- --nocapture
```

Expected: clippy clean, unit tests pass.

- [ ] **Step 5: Run the integration suite**

```powershell
cd server
cargo test --profile release --features load-dynamic -p test_integration -- --nocapture
```

Expected: the whole suite passes, including `test_takeout_import_flow` (albums split over ZIPs, cross-ZIP sidecar, dedupe of the existing video, cleanup).

- [ ] **Step 6: Commit**

```powershell
git add server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs server/crates/test_integration/src/tests/test_takeout.rs
git commit -m "feat: import takeout media by streaming it out of the zips"
```

---

### Task 5: Corrupt-entry integration test

**Files:**
- Test: `server/crates/test_integration/src/tests/test_takeout.rs:28-43` (constants/helpers), `:46-117` (`create_takeout_zips`), `:467-627` (`test_takeout_import_flow`)

**Interfaces:**
- Consumes: `archive::copy_entries` skip policy from Task 3; `bulk_enqueue_full_ingest` from Task 4.
- Produces: no code.

- [ ] **Step 1: Add a third ZIP with one corrupt entry and one good entry**

Add constants `CORRUPT_PHOTO: &str = "takeout_corrupt(1).jpg";` and `GOOD_AFTER_CORRUPT: &str = "takeout_after_corrupt(1).png";`. Add a helper:

```rust
/// A ZIP whose first entry has valid metadata but one flipped data byte, so reading it fails CRC.
fn create_corrupt_zip(folder: &Path) -> Result<PathBuf> {
    let payload = b"unique-corrupt-payload-0123456789";
    let path = folder.join("takeout-003.zip");
    let file = std::fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file(
        format!("Takeout/Google Photos/Broken Album/{CORRUPT_PHOTO}"),
        stored,
    )?;
    zip.write_all(payload)?;
    zip.start_file(
        format!("Takeout/Google Photos/Broken Album/{GOOD_AFTER_CORRUPT}"),
        SimpleFileOptions::default(),
    )?;
    zip.write_all(NO_METADATA_PNG)?;
    zip.finish()?;

    let mut bytes = std::fs::read(&path)?;
    let start = bytes
        .windows(payload.len())
        .position(|window| window == payload)
        .expect("corrupt payload should be stored verbatim");
    bytes[start + payload.len() / 2] ^= 0xFF;
    std::fs::write(&path, bytes)?;
    Ok(path)
}
```

In `create_takeout_zips`, create it with `let zip3 = create_corrupt_zip(folder)?;` and return `(zip1, zip2, zip3)`. Update the one call site to destructure three paths and upload all three.

- [ ] **Step 2: Update the flow expectations**

- `assert_eq!(zip_count, 3);`
- Upload `zip3_bytes`.
- `expected_ids` now sorts and contains three ids.
- After the job, assert the corrupt file never became media and left no part file:

```rust
    let corrupt = MediaItemStore::find_existing_filenames_map(
        &context.pool,
        user_id,
        &[CORRUPT_PHOTO.to_owned()],
    )
    .await?;
    assert!(corrupt.is_empty(), "a corrupt entry must be skipped");

    assert!(
        !context
            .settings
            .ingest
            .media_root
            .join("takeout")
            .join(format!("{CORRUPT_PHOTO}.part"))
            .exists(),
        "a skipped entry must not leave a partial file"
    );
```

Use existing store calls rather than new `sqlx::query_scalar!` strings, so `server/.sqlx` gains no entry. Add the imports:

```rust
use common_services::database::album_store::AlbumStore;
use common_services::database::media_item_store::MediaItemStore;
```

- Assert the good entry from the same ZIP still imported: `wait_for_media_item(context, user_id, GOOD_AFTER_CORRUPT).await?;` and that "Broken Album" exists (the folder was registered at index time even though one entry was unreadable):

```rust
    assert!(
        AlbumStore::find_by_name(&context.pool, user_id, "Broken Album")
            .await?
            .is_some(),
        "the album folder is still registered when one of its entries is unreadable"
    );
```

- [ ] **Step 3: Run the integration suite**

Same command as Task 4 Step 5. Expected: PASS, including the new assertions.

- [ ] **Step 4: Commit**

```powershell
git add server/crates/test_integration/src/tests/test_takeout.rs
git commit -m "test: cover skipping a corrupt takeout entry"
```

---

### Task 6: Final gate

**Files:** none.

- [ ] **Step 1: Full local gate**

From `server/`:

```powershell
just fmt
just clippy
just test
```

Expected: formatting clean, clippy clean, whole integration suite green. `just test` rewrites `crates/test_integration/assets/static-cache/tag-vocab/*.bin`; if only those binaries changed, `git checkout -- server/crates/test_integration/assets/static-cache/tag-vocab`.

- [ ] **Step 2: Confirm nothing else moved**

```powershell
git status --short
```

Expected: only the files listed in this plan. No `.sqlx` changes. No `web/tests/perf` result needed unless a later change touches `web/` beyond `nginx.conf` (the perf gate looks at `web/**`; if CI flags the nginx change, add the standard perf result file from `web/tests/perf`).

- [ ] **Step 3: Commit any formatter output**

```powershell
git add -A
git commit -m "chore: format"
```

Skip this commit if `git status` is empty.
