use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSyncParams {
    /// Epoch millis of the previous sync's `sync_timestamp`. Omit for a full snapshot.
    pub since: Option<i64>,
}
