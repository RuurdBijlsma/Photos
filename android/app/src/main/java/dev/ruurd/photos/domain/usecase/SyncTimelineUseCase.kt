package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.network.SyncTimestampStore
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.proto.CompactTimelineItem
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.IOException
import java.time.Instant
import java.time.ZoneOffset

class SyncTimelineUseCase(
    private val apiService: PhotosApiService,
    private val mediaRepository: MediaRepository,
    private val sessionManager: SyncTimestampStore,
    private val dispatchers: AppDispatchers
) {
    private val syncMutex = Mutex()

    private val _syncState = MutableStateFlow(SyncState.IDLE)
    val syncState: StateFlow<SyncState> = _syncState.asStateFlow()

    suspend operator fun invoke(forceFullSnapshot: Boolean = false): Result<TimelineSyncResponse> =
        withContext(dispatchers.io) {
            syncMutex.withLock {
                _syncState.value = SyncState.SYNCING
                try {
                    val latestSyncTimestamp = sessionManager.latestSyncTimestamp.value
                    val since = if (forceFullSnapshot || latestSyncTimestamp <= 0L) {
                        null
                    } else {
                        // 60s overlap prevents missing concurrent server ingests committed after sync starts
                        maxOf(0L, latestSyncTimestamp - 60_000L)
                    }

                    val response = apiService.getTimelineSync(since = since)
                    if (!response.isSuccessful) {
                        _syncState.value = if (response.code() in 500..599) {
                            SyncState.SERVER_UNREACHABLE
                        } else {
                            SyncState.IDLE
                        }
                        return@withLock Result.failure(
                            Exception("Sync request failed: HTTP ${response.code()}")
                        )
                    }

                    val syncResponse = response.body()
                    if (syncResponse == null) {
                        _syncState.value = SyncState.SERVER_UNREACHABLE
                        return@withLock Result.failure(Exception("Sync response body is null"))
                    }

                    mediaRepository.withTransaction {
                        val existingCloudIds = mediaRepository.getAllCloudIds().toHashSet()

                        // 1. Process deletions (full snapshot diff or delta deleted_ids)
                        val cloudIdsToRemove = if (syncResponse.isFullSnapshot) {
                            val syncedCloudIds = HashSet<String>(syncResponse.upsertedCount).apply {
                                for (item in syncResponse.upsertedList) {
                                    add(item.id)
                                }
                            }
                            existingCloudIds.filter { it !in syncedCloudIds }
                        } else {
                            syncResponse.deletedIdsList
                        }

                        for (cloudId in cloudIdsToRemove) {
                            mediaRepository.deleteCloudOnlyByCloudId(cloudId)
                            mediaRepository.transitionToLocalOnly(
                                cloudId = cloudId,
                                // TODO(Phase 1B): derive from device_folder.show_in_timeline
                                isVisible = true
                            )
                        }

                        // 2. Process upserts in chunked batches (updating canonical + any duplicate local rows)
                        val itemsToUpsert = ArrayList<MediaEntity>(syncResponse.upsertedCount)
                        for (item in syncResponse.upsertedList) {
                            val existingRows = if (item.id in existingCloudIds) {
                                mediaRepository.getAllByCloudId(item.id)
                            } else {
                                emptyList()
                            }
                            if (existingRows.isEmpty()) {
                                itemsToUpsert.add(toMediaEntity(item, existing = null))
                            } else {
                                for (existing in existingRows) {
                                    itemsToUpsert.add(toMediaEntity(item, existing = existing))
                                }
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
                } catch (e: CancellationException) {
                    if (_syncState.value == SyncState.SYNCING) {
                        _syncState.value = SyncState.IDLE
                    }
                    throw e
                } catch (e: IOException) {
                    _syncState.value = SyncState.SERVER_UNREACHABLE
                    Result.failure(e)
                } catch (e: Exception) {
                    _syncState.value = SyncState.IDLE
                    Result.failure(e)
                }
            }
        }

    private fun toMediaEntity(item: CompactTimelineItem, existing: MediaEntity?): MediaEntity {
        val yearMonth = computeYearMonth(item.sortTimestamp, item.timezoneOffsetMinutes)
        val durationMs = if (item.hasDurationMs()) item.durationMs else null

        return existing?.copy(
            sortTimestamp = item.sortTimestamp,
            timezoneOffsetMinutes = item.timezoneOffsetMinutes,
            yearMonth = yearMonth,
            ratio = item.ratio,
            isVideo = item.isVideo,
            durationMs = durationMs,
            hasThumbnails = item.hasThumbnails,
            sizeBytes = item.sizeBytes
        ) ?: MediaEntity(
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
            durationMs = durationMs,
            hasThumbnails = item.hasThumbnails,
            syncStatus = SyncStatus.CLOUD_ONLY
        )
    }

    private fun computeYearMonth(sortTimestamp: Long, timezoneOffsetMinutes: Int): Int {
        val epochWithOffset = sortTimestamp + (timezoneOffsetMinutes * 60_000L)
        val instant = Instant.ofEpochMilli(epochWithOffset)
        val zdt = instant.atZone(ZoneOffset.UTC)
        return zdt.year * 100 + zdt.monthValue
    }
}