package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.proto.CompactTimelineItem
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import java.time.Instant
import java.time.ZoneOffset

class SyncTimelineUseCase(
    private val apiService: PhotosApiService,
    private val mediaRepository: MediaRepository,
    private val sessionManager: SessionManager,
    private val dispatchers: AppDispatchers
) {
    private val _syncState = MutableStateFlow(SyncState.IDLE)
    val syncState: StateFlow<SyncState> = _syncState.asStateFlow()

    suspend operator fun invoke(forceFullSnapshot: Boolean = false): Result<TimelineSyncResponse> =
        withContext(dispatchers.io) {
            _syncState.value = SyncState.SYNCING
            try {
                val latestSyncTimestamp = sessionManager.latestSyncTimestamp.value
                val since = if (forceFullSnapshot || latestSyncTimestamp <= 0L) {
                    null
                } else {
                    maxOf(0L, latestSyncTimestamp - 60_000L) // 60s overlap for concurrent ingest safety
                }

                val response = apiService.getTimelineSync(since = since)
                if (!response.isSuccessful) {
                    val state = if (response.code() in 500..599) {
                        SyncState.SERVER_UNREACHABLE
                    } else {
                        SyncState.IDLE
                    }
                    _syncState.value = state
                    return@withContext Result.failure(Exception("Sync request failed: HTTP ${response.code()}"))
                }

                val syncResponse = response.body()
                    ?: return@withContext Result.failure(Exception("Sync response body is null"))

                mediaRepository.withTransaction {
                    // 1. Process deletions
                    if (syncResponse.isFullSnapshot) {
                        // Full snapshot reconciliation: diff local cloud_ids against received snapshot
                        val syncedCloudIds = syncResponse.upsertedList.map { it.id }.toSet()
                        val existingCloudIds = mediaRepository.getAllCloudIds()
                        val staleCloudIds = existingCloudIds.filter { it !in syncedCloudIds }

                        for (staleId in staleCloudIds) {
                            val existing = mediaRepository.getByCloudId(staleId)
                            if (existing != null) {
                                if (existing.localContentUri != null) {
                                    // Local copy exists: revert to LOCAL_ONLY, do not re-upload immediately
                                    mediaRepository.transitionToLocalOnly(
                                        cloudId = staleId,
                                        isVisible = true
                                    )
                                } else {
                                    // Pure cloud copy: delete row
                                    mediaRepository.deleteCloudOnlyByCloudId(staleId)
                                }
                            }
                        }
                    } else {
                        // Delta sync deletions
                        for (deletedId in syncResponse.deletedIdsList) {
                            val existing = mediaRepository.getByCloudId(deletedId)
                            if (existing != null) {
                                if (existing.localContentUri != null) {
                                    mediaRepository.transitionToLocalOnly(
                                        cloudId = deletedId,
                                        isVisible = true
                                    )
                                } else {
                                    mediaRepository.deleteCloudOnlyByCloudId(deletedId)
                                }
                            }
                        }
                    }

                    // 2. Process upserts in chunked batches
                    val itemsToUpsert = mutableListOf<MediaEntity>()

                    for (item in syncResponse.upsertedList) {
                        val yearMonth = computeYearMonth(item.sortTimestamp, item.timezoneOffsetMinutes)
                        val existing = mediaRepository.getByCloudId(item.id)

                        if (existing != null) {
                            // Update existing row
                            val updated = existing.copy(
                                sortTimestamp = item.sortTimestamp,
                                timezoneOffsetMinutes = item.timezoneOffsetMinutes,
                                yearMonth = yearMonth,
                                ratio = item.ratio,
                                isVideo = item.isVideo,
                                durationMs = if (item.hasDurationMs()) item.durationMs else null,
                                hasThumbnails = item.hasThumbnails,
                                sizeBytes = item.sizeBytes
                            )
                            itemsToUpsert.add(updated)
                        } else {
                            // New cloud item
                            val entity = MediaEntity(
                                id = "cloud_${item.id}",
                                cloudId = item.id,
                                localContentUri = null,
                                bucketId = null,
                                bucketName = null,
                                relativePath = null,
                                isVisibleInTimeline = true,
                                autoBackupSuppressed = false,
                                sizeBytes = item.sizeBytes,
                                ratio = item.ratio,
                                sortTimestamp = item.sortTimestamp,
                                timezoneOffsetMinutes = item.timezoneOffsetMinutes,
                                yearMonth = yearMonth,
                                isVideo = item.isVideo,
                                durationMs = if (item.hasDurationMs()) item.durationMs else null,
                                hasThumbnails = item.hasThumbnails,
                                syncStatus = SyncStatus.CLOUD_ONLY
                            )
                            itemsToUpsert.add(entity)
                        }
                    }

                    if (itemsToUpsert.isNotEmpty()) {
                        // Upsert in batches of 500 to stay well below SQLite bind limits
                        itemsToUpsert.chunked(500).forEach { chunk ->
                            mediaRepository.upsertAll(chunk)
                        }
                    }

                    // 3. Persist server sync timestamp
                    sessionManager.setLatestSyncTimestamp(syncResponse.syncTimestamp)
                }

                _syncState.value = SyncState.IDLE
                Result.success(syncResponse)
            } catch (e: Exception) {
                _syncState.value = SyncState.SERVER_UNREACHABLE
                Result.failure(e)
            }
        }

    private fun computeYearMonth(sortTimestamp: Long, timezoneOffsetMinutes: Int): Int {
        val epochWithOffset = sortTimestamp + (timezoneOffsetMinutes * 60_000L)
        val instant = Instant.ofEpochMilli(epochWithOffset)
        val zdt = instant.atZone(ZoneOffset.UTC)
        return zdt.year * 100 + zdt.monthValue
    }
}
