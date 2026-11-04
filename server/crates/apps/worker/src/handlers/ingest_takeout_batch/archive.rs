use chrono::{DateTime, Utc};
use color_eyre::Result;
use sanitize_filename::sanitize;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tracing::{info, warn};
use zip::ZipArchive;

/// A single entry in one uploaded Takeout ZIP.
#[derive(Debug)]
pub(super) struct EntryRef {
    pub upload_id: String,
    pub index: usize,
}

/// A media file found in the central directory, with every album folder that holds it.
#[derive(Debug)]
pub(super) struct MediaEntry {
    pub entry: EntryRef,
    pub folder: String,
    pub filename: String,
    pub size: u64,
    pub albums: HashSet<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct AlbumMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default)]
pub(super) struct TakeoutIndex {
    pub albums: BTreeMap<String, AlbumMetadata>,
    /// Sanitized filename -> first occurrence.
    pub media: HashMap<String, MediaEntry>,
    /// (album folder, lowercased sidecar name) -> `taken_at`, `None` when unparseable.
    pub sidecars: HashMap<(String, String), Option<DateTime<Utc>>>,
}

impl TakeoutIndex {
    pub fn merge(&mut self, other: Self) {
        for (folder, metadata) in other.albums {
            let existing = self.albums.entry(folder).or_default();
            if existing.title.is_none() {
                existing.title = metadata.title;
            }
            if existing.description.is_none() {
                existing.description = metadata.description;
            }
        }
        for (filename, media) in other.media {
            if let Some(existing) = self.media.get_mut(&filename) {
                existing.albums.extend(media.albums);
            } else {
                self.media.insert(filename, media);
            }
        }
        for (key, timestamp) in other.sidecars {
            self.sidecars.entry(key).or_insert(timestamp);
        }
    }
}

/// Splits an unsafe or unexpected entry name into its components, and `None` for a name that
/// must not be trusted: absolute paths, drive/ADS prefixes, and `.`/`..`.
///
/// `ZipFile::enclosed_name()` cannot be used instead: zip 9.2.2 rejects every component ending in
/// a space on Windows, which is every real Takeout album folder.
fn safe_components(name: &str) -> Option<Vec<String>> {
    if name.starts_with(['/', '\\']) {
        return None;
    }
    let components: Vec<String> = name
        .split(['/', '\\'])
        .map(str::trim)
        .filter(|component| !component.is_empty())
        .map(str::to_owned)
        .collect();
    let unsafe_component = components
        .iter()
        .any(|component| component.contains(':') || component == "." || component == "..");
    if unsafe_component {
        return None;
    }
    Some(components)
}

/// Album folder and file name of a Takeout media entry, e.g.
/// `Takeout/Google Photos/Vacation 2023 /a.jpg` -> (`Vacation 2023`, `a.jpg`).
fn takeout_parts(name: &str) -> Option<(String, String)> {
    let mut components = safe_components(name)?;
    if components.first().is_some_and(|first| first == "Takeout") {
        components.remove(0);
    }
    match components.as_slice() {
        [root, album, file] if root == "Google Photos" => Some((album.clone(), file.clone())),
        _ => None,
    }
}

/// Reads at most 1 MiB, so one oversized or unreadable JSON entry cannot exhaust memory or fail
/// the whole archive.
fn read_small_json(entry: &mut impl Read) -> Option<String> {
    let mut content = String::new();
    entry.take(1 << 20).read_to_string(&mut content).ok()?;
    Some(content)
}

#[derive(Debug, Deserialize)]
struct SidecarTimestamp {
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SidecarJson {
    #[serde(rename = "photoTakenTime")]
    photo_taken_time: Option<SidecarTimestamp>,
    #[serde(rename = "creationTime")]
    creation_time: Option<SidecarTimestamp>,
}

fn parse_sidecar_timestamp(content: &str) -> Option<DateTime<Utc>> {
    let parsed: SidecarJson = serde_json::from_str(content).ok()?;

    let ts_str = parsed
        .photo_taken_time
        .and_then(|t| t.timestamp)
        .or_else(|| parsed.creation_time.and_then(|t| t.timestamp))?;

    let epoch_secs = ts_str.parse::<i64>().ok()?;
    DateTime::from_timestamp(epoch_secs, 0)
}

/// Splits a trailing Takeout duplicate counter, e.g. `IMG_0001(2)` -> (`IMG_0001`, `2`).
fn split_duplicate_counter(stem: &str) -> (&str, Option<&str>) {
    if let Some(open) = stem.rfind('(')
        && let Some(counter) = stem[open + 1..].strip_suffix(')')
        && !counter.is_empty()
        && counter.bytes().all(|b| b.is_ascii_digit())
    {
        return (&stem[..open], Some(counter));
    }
    (stem, None)
}

/// Candidate sidecar names for a media file, most specific first.
///
/// Takeout does not always name the sidecar after the media file exactly: duplicate counters
/// may live on either side, the media extension can be omitted, and edited copies often
/// inherit the original's sidecar.
fn sidecar_candidates(filename: &str) -> Vec<String> {
    let path = Path::new(filename);
    let (Some(stem), Some(ext)) = (
        path.file_stem().and_then(|s| s.to_str()),
        path.extension().and_then(|e| e.to_str()),
    ) else {
        return Vec::new();
    };

    let (base_stem, counter) = split_duplicate_counter(stem);
    let mut candidates = vec![
        format!("{filename}.supplemental-metadata.json"),
        format!("{filename}.json"),
    ];

    if let Some(counter) = counter {
        candidates.push(format!(
            "{base_stem}.{ext}.supplemental-metadata({counter}).json"
        ));
        candidates.push(format!("{base_stem}.supplemental-metadata({counter}).json"));
    }

    candidates.push(format!("{base_stem}.{ext}.supplemental-metadata.json"));
    candidates.push(format!("{base_stem}.supplemental-metadata.json"));
    candidates.push(format!("{base_stem}.{ext}.json"));
    candidates.push(format!("{base_stem}.json"));

    if let Some(edited_base) = base_stem.strip_suffix("-edited") {
        candidates.push(format!("{edited_base}.{ext}.supplemental-metadata.json"));
        candidates.push(format!("{edited_base}.{ext}.json"));
        candidates.push(format!("{edited_base}.supplemental-metadata.json"));
    }

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

/// Indexes one ZIP's central directory and JSON entries without decompressing any media, so a
/// 148 GB batch needs no extra disk space.
pub(super) fn index_archive(
    index: &mut TakeoutIndex,
    upload_id: &str,
    zip_path: &Path,
    is_media: impl Fn(&Path) -> bool,
) -> Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut archive_albums: HashSet<String> = HashSet::new();
    let mut media_entries = 0_usize;
    let mut sidecar_entries = 0_usize;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(entry) => entry,
            Err(error) => {
                warn!("Skipping unreadable Takeout entry {i} in {upload_id}: {error}");
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }

        // The name is copied out so the entry can still be read below.
        let entry_name = match entry.name() {
            Ok(name) => name.into_owned(),
            Err(error) => {
                warn!("Skipping Takeout entry {i} in {upload_id} with an unreadable name: {error}");
                continue;
            }
        };
        let Some((folder, file_name)) = takeout_parts(&entry_name) else {
            continue;
        };
        let size = entry.size();
        archive_albums.insert(folder.clone());
        index.albums.entry(folder.clone()).or_default();

        if Path::new(&file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            let Some(content) = read_small_json(&mut entry) else {
                continue;
            };
            if file_name.eq_ignore_ascii_case("metadata.json") {
                let Ok(metadata) = serde_json::from_str::<AlbumMetadata>(&content) else {
                    continue;
                };
                let album = index.albums.entry(folder).or_default();
                if album.title.is_none() {
                    album.title = metadata
                        .title
                        .map(|title| title.trim().to_owned())
                        .filter(|title| !title.is_empty());
                }
                if album.description.is_none() {
                    album.description = metadata
                        .description
                        .map(|description| description.trim().to_owned())
                        .filter(|description| !description.is_empty());
                }
            } else {
                index
                    .sidecars
                    .entry((folder, sanitize(&file_name).to_lowercase()))
                    .or_insert_with(|| parse_sidecar_timestamp(&content));
                sidecar_entries += 1;
            }
        } else if is_media(Path::new(&file_name)) {
            let media = index
                .media
                .entry(sanitize(&file_name))
                .or_insert_with(|| MediaEntry {
                    entry: EntryRef {
                        upload_id: upload_id.to_owned(),
                        index: i,
                    },
                    folder: folder.clone(),
                    filename: sanitize(&file_name),
                    size,
                    albums: HashSet::new(),
                });
            media.albums.insert(folder);
            media_entries += 1;
        }
    }

    info!(
        "Indexed ZIP {upload_id}: {} album(s), {media_entries} media entry(ies), {sidecar_entries} sidecar(s)",
        archive_albums.len()
    );
    Ok(())
}

pub(super) struct CopyRequest {
    pub entry_index: usize,
    pub filename: String,
    pub size: u64,
    pub fallback_taken_at: Option<DateTime<Utc>>,
}

pub(super) enum CopyOutcome {
    Copied {
        filename: String,
        size: u64,
        fallback_taken_at: Option<DateTime<Utc>>,
    },
    Skipped {
        filename: String,
        reason: String,
    },
}

/// Copies the requested entries out of one ZIP into `dest_folder`, skipping (never failing the
/// batch on) an unreadable entry. The archive itself is opened once, so a batch stays one pass.
pub(super) fn copy_entries(
    zip_path: &Path,
    requests: &[CopyRequest],
    dest_folder: &Path,
) -> Result<Vec<CopyOutcome>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut outcomes = Vec::with_capacity(requests.len());

    for request in requests {
        let part_path = dest_folder.join(format!("{}.part", request.filename));
        let dest_path = dest_folder.join(&request.filename);
        let skipped = |reason: String| CopyOutcome::Skipped {
            filename: request.filename.clone(),
            reason,
        };

        // A `.part` beside the destination is invisible to the watcher: `takeout/` is managed.
        let outcome = match copy_entry(&mut archive, request.entry_index, &part_path) {
            Ok(size) if size == request.size => {
                if let Err(error) = replace_file(&part_path, &dest_path) {
                    skipped(format!(
                        "could not replace {}: {error}",
                        dest_path.display()
                    ))
                } else {
                    CopyOutcome::Copied {
                        filename: request.filename.clone(),
                        size: request.size,
                        fallback_taken_at: request.fallback_taken_at,
                    }
                }
            }
            Ok(size) => skipped(format!("copied {size} bytes, expected {}", request.size)),
            Err(reason) => skipped(reason),
        };

        if matches!(outcome, CopyOutcome::Skipped { .. }) {
            let _ = std::fs::remove_file(&part_path);
        }
        outcomes.push(outcome);
    }

    Ok(outcomes)
}

/// Streams one entry into `part_path` and returns the copied byte count.
fn copy_entry(
    archive: &mut ZipArchive<BufReader<File>>,
    entry_index: usize,
    part_path: &Path,
) -> Result<u64, String> {
    let mut entry = archive
        .by_index(entry_index)
        .map_err(|error| format!("could not read entry {entry_index}: {error}"))?;
    let mut part = File::create(part_path)
        .map_err(|error| format!("could not create {}: {error}", part_path.display()))?;
    let copied = std::io::copy(&mut entry, &mut part)
        .map_err(|error| format!("could not copy entry {entry_index}: {error}"))?;
    part.sync_all()
        .map_err(|error| format!("could not flush {}: {error}", part_path.display()))?;
    Ok(copied)
}

/// Windows refuses `rename` onto an existing file, so a leftover from a failed attempt is
/// replaced instead of producing a `_1`-suffixed duplicate.
fn replace_file(part_path: &Path, dest_path: &Path) -> std::io::Result<()> {
    if let Err(error) = std::fs::remove_file(dest_path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(error);
    }
    std::fs::rename(part_path, dest_path)
}

/// The sidecar date for a media file: the first candidate name that the index holds decides,
/// even when that sidecar is unparseable.
pub(super) fn sidecar_timestamp(
    index: &TakeoutIndex,
    folder: &str,
    filename: &str,
) -> Option<DateTime<Utc>> {
    // Takeout is inconsistent about extension case (`google.PNG` vs `google.png....json`).
    for candidate in sidecar_candidates(filename) {
        let key = (folder.to_owned(), candidate.to_lowercase());
        if let Some(timestamp) = index.sidecars.get(&key) {
            return *timestamp;
        }
    }
    None
}

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
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
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
        let sidecar = format!(r#"{{"photoTakenTime": {{"timestamp": "{SIDECAR_TS}"}}}}"#);
        build_zip(
            &zip_path,
            &[
                (
                    "Takeout/Google Photos/Vacation 2023 /metadata.json",
                    br#"{"title": "Summer Vacation", "description": "Summer holiday"}"#,
                ),
                (
                    "Takeout/Google Photos/Vacation 2023 /photo.jpg",
                    b"jpeg-bytes",
                ),
                (
                    "Takeout/Google Photos/Vacation 2023 /photo.jpg.supplemental-metadata.json",
                    sidecar.as_bytes(),
                ),
                (
                    "Takeout/Google Photos/Photos from 2023/photo.jpg",
                    b"jpeg-bytes",
                ),
                (
                    "Takeout/Google Photos/Photos from 2023/notes.txt",
                    b"ignored",
                ),
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

        build_zip(
            &zip_path,
            &[("Takeout/Google Photos/Album/photo.jpg", b"fresh")],
        );
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
        assert_eq!(
            std::fs::read(dest.join("photo.jpg")).expect("read"),
            b"fresh"
        );
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

    #[test]
    fn duplicate_filename_keeps_the_first_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("takeout.zip");
        build_zip(
            &zip_path,
            &[
                ("Takeout/Google Photos/Album A/photo.jpg", b"first"),
                (
                    "Takeout/Google Photos/Album B/photo.jpg",
                    b"second-and-longer",
                ),
            ],
        );

        let mut index = TakeoutIndex::default();
        index_archive(&mut index, "upload-1", &zip_path, is_media).expect("index");

        // The copy pass compares the copied length against `size`, so `size` must describe the
        // entry that `entry` points at, not a later occurrence with the same name.
        let media = &index.media["photo.jpg"];
        assert_eq!(media.entry.index, 0);
        assert_eq!(media.size, 5);
        assert!(media.albums.contains("Album A"));
        assert!(media.albums.contains("Album B"));
    }

    #[test]
    fn sidecar_candidates_cover_takeout_naming_quirks() {
        assert_eq!(
            sidecar_candidates("photo.jpg")[0],
            "photo.jpg.supplemental-metadata.json"
        );
        // Counter on the media file and on the sidecar.
        assert!(
            sidecar_candidates("2015-03-22(2).jpg")
                .contains(&"2015-03-22.jpg.supplemental-metadata(2).json".to_owned())
        );
        // Counter only on the sidecar, without the media extension.
        assert!(
            sidecar_candidates("2013-08-11(1).png")
                .contains(&"2013-08-11.supplemental-metadata(1).json".to_owned())
        );
        // Sidecar without the media extension.
        assert!(sidecar_candidates("bg.png").contains(&"bg.supplemental-metadata.json".to_owned()));
        // Edited copies fall back to the original's sidecar.
        assert!(
            sidecar_candidates("photo-edited.jpg")
                .contains(&"photo.jpg.supplemental-metadata.json".to_owned())
        );
        // The exact sidecar always wins over the counter variants.
        assert_eq!(
            sidecar_candidates("photo(1).jpg")[0],
            "photo(1).jpg.supplemental-metadata.json"
        );
    }

    #[test]
    fn sidecar_lookup_ignores_extension_case() {
        let folder = "Photos from 2013";
        let timestamp = DateTime::from_timestamp(SIDECAR_TS, 0).expect("timestamp");
        let mut index = TakeoutIndex::default();
        index.sidecars.insert(
            (
                folder.to_owned(),
                "google.png.supplemental-metadata.json".to_owned(),
            ),
            Some(timestamp),
        );

        // `google-edited.png` inherits the sidecar of `google.PNG`.
        assert_eq!(
            sidecar_timestamp(&index, folder, "google-edited.png"),
            Some(timestamp)
        );
    }

    fn refute_zip_extraction(dir: &Path) {
        assert_eq!(
            std::fs::read_dir(dir)
                .expect("read dir")
                .filter(|entry| entry
                    .as_ref()
                    .expect("entry")
                    .file_type()
                    .expect("type")
                    .is_dir())
                .count(),
            0,
            "indexing must not create directories or extract media"
        );
    }

    fn assert_no_part_files(dest: &Path) {
        assert!(
            !std::fs::read_dir(dest).expect("read dir").any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".part")),
            "a failed or finished copy must not leave .part files"
        );
    }
}
