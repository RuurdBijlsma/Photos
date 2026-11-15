package dev.ruurd.photos.testing

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf

/** In-memory repository mirroring the DAO semantics the use cases rely on. */
class FakeMediaRepository : MediaRepository {
    val items = mutableMapOf<String, MediaEntity>()

    /** bucket_id -> show_in_timeline, i.e. what the DAO reads from `device_folder`. */
    val folderVisibility = mutableMapOf<String, Boolean>()

    override fun getTimelineStage1(limitMonths: Int): Flow<List<MediaEntity>> =
        flowOf(items.values.toList())

    override fun getTimelineStage2(): Flow<List<MediaEntity>> = flowOf(items.values.toList())

    override fun countVisible(): Flow<Int> = flowOf(items.values.count { it.isVisibleInTimeline })

    override fun countTotal(): Flow<Int> = flowOf(items.size)

    override fun countLocal(): Flow<Int> = flowOf(items.values.count { it.localContentUri != null })

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
        items.values.filter { it.sizeBytes == sizeBytes && it.syncStatus == syncStatus }

    override suspend fun upsert(media: MediaEntity) {
        items[media.id] = media
    }

    override suspend fun upsertAll(mediaList: List<MediaEntity>) {
        mediaList.forEach { items[it.id] = it }
    }

    override suspend fun deleteById(id: String) {
        items.remove(id)
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
    }

    override suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        val targetIds = cloudIds.toHashSet()
        items.values
            .filter { it.cloudId in targetIds && it.localContentUri == null }
            .map { it.id }
            .forEach { items.remove(it) }
    }

    override suspend fun deleteByCloudId(cloudId: String) {
        items.values.filter { it.cloudId == cloudId }.map { it.id }.forEach { items.remove(it) }
    }

    override suspend fun deleteByCloudIds(cloudIds: List<String>) {
        cloudIds.forEach { deleteByCloudId(it) }
    }

    override suspend fun <T> withTransaction(block: suspend () -> T): T = block()
}