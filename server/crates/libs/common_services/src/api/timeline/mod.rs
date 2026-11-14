pub mod interfaces;
pub mod service;
pub mod sync;

/// How long permanently deleted media ids stay in `deleted_media_item` for timeline delta sync.
///
/// A client syncing with a `since` older than this gets a full snapshot instead, because the
/// tombstones it needs may already be pruned.
pub const TOMBSTONE_RETENTION_DAYS: i64 = 90;
