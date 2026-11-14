pub mod interfaces;
pub mod service;

/// How long permanently deleted media ids stay in `deleted_media_item` for mobile delta sync.
///
/// A client syncing with a `since` older than this gets a full snapshot instead, because the
/// tombstones it needs may already be pruned.
pub const TOMBSTONE_RETENTION_DAYS: i64 = 90;
