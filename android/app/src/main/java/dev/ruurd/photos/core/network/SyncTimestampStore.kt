package dev.ruurd.photos.core.network

import kotlinx.coroutines.flow.StateFlow

/** Persisted cursor for delta timeline sync. */
interface SyncTimestampStore {
    val latestSyncTimestamp: StateFlow<Long>
    suspend fun setLatestSyncTimestamp(timestamp: Long)
}
