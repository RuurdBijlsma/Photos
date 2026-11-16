package dev.ruurd.photos.testing

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map

/** In-memory repository mirroring the DAO semantics the use cases rely on. */
class FakeMediaRepository : MediaRepository {
    val items = mutableMapOf<String, MediaEntity>()

    /** bucket_id -> show_in_timeline, i.e. what the DAO reads from `device_folder`. */
    val folderVisibility = mutableMapOf<String, Boolean>()

    private val timelineFlow = MutableStateFlow<List<TimelineMediaProjection>>(emptyList())

    override fun getTimelineStage1(limitMonths: Int): Flow<List<TimelineMediaProjection>> =
        timelineFlow.map { list ->
            val topMonths = list.asSequence()
                .map { it.yearMonth }
                .distinct()
                .take(limitMonths)
                .toHashSet()
            list.filter { it.yearMonth in topMonths }
        }

    override fun getTimelineStage2(): Flow<List<TimelineMediaProjection>> = timelineFlow

    override suspend fun getById(id: String): MediaEntity? = items[id]

    override suspend fun getByCloudId(cloudId: String): MediaEntity? =
        items.values.firstOrNull { it.cloudId == cloudId }

    override suspend fun getAllByCloudId(cloudId: String): List<MediaEntity> =
        items.values.filter { it.cloudId == cloudId }

    override suspend fun getLocalRows(isVideo: Boolean): List<MediaEntity> =
        items.values.filter { it.localContentUri != null && it.isVideo == isVideo }

    override suspend fun getAllCloudIds(): List<String> =
        items.values.mapNotNull { it.cloudId }.distinct()

    override suspend fun findBySizeAndStatus(
        sizeBytes: Long,
        syncStatus: SyncStatus
    ): List<MediaEntity> =
        items.values
            .filter { it.sizeBytes == sizeBytes && it.syncStatus == syncStatus }
            .sortedWith(
                compareByDescending<MediaEntity> { it.isVisibleInTimeline }
                    .thenBy { it.id }
            )

    override suspend fun upsert(media: MediaEntity) {
        items[media.id] = media
        emitTimeline()
    }

    override suspend fun upsertAll(mediaList: List<MediaEntity>) {
        mediaList.forEach { items[it.id] = it }
        emitTimeline()
    }

    override suspend fun deleteById(id: String) {
        items.remove(id)
        emitTimeline()
    }

    override suspend fun transitionToLocalOnlyByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        val targetIds = cloudIds.toHashSet()
        items.values
            .filter { it.cloudId in targetIds && it.localContentUri != null }
            .forEach {
                items[it.id] = it.copy(
                    cloudId = null,
                    syncStatus = SyncStatus.LOCAL_ONLY,
                    autoBackupSuppressed = true,
                    // Mirrors the DAO's COALESCE(device_folder.show_in_timeline, 1) subquery.
                    isVisibleInTimeline = it.bucketId?.let(folderVisibility::get) ?: true
                )
            }
        emitTimeline()
    }

    override suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        val targetIds = cloudIds.toHashSet()
        items.values
            .filter { it.cloudId in targetIds && it.localContentUri == null }
            .map { it.id }
            .forEach { items.remove(it) }
        emitTimeline()
    }

    override suspend fun deleteByCloudId(cloudId: String) {
        items.values.filter { it.cloudId == cloudId }.map { it.id }.forEach { items.remove(it) }
        emitTimeline()
    }

    override suspend fun deleteByCloudIds(cloudIds: List<String>) {
        val targetIds = cloudIds.toHashSet()
        items.values.filter { it.cloudId in targetIds }.map { it.id }.forEach { items.remove(it) }
        emitTimeline()
    }

    override suspend fun clearAll() {
        items.clear()
        emitTimeline()
    }

    override suspend fun <T> withTransaction(block: suspend () -> T): T = block()

    private fun emitTimeline() {
        timelineFlow.value = items.values
            .filter { it.isVisibleInTimeline }
            .sortedWith(
                compareByDescending<MediaEntity> { it.yearMonth }
                    .thenByDescending { it.sortTimestamp }
                    .thenByDescending { it.id }
            )
            .map {
                TimelineMediaProjection(
                    id = it.id,
                    cloudId = it.cloudId,
                    localContentUri = it.localContentUri,
                    yearMonth = it.yearMonth,
                    ratio = it.ratio,
                    isVideo = it.isVideo,
                    durationMs = it.durationMs,
                    hasThumbnails = it.hasThumbnails,
                    syncStatus = it.syncStatus,
                    sortTimestamp = it.sortTimestamp,
                    timezoneOffsetMinutes = it.timezoneOffsetMinutes
                )
            }
    }
}