package dev.ruurd.photos.data.repository

import androidx.room.withTransaction
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.dao.MediaDao
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOn

class MediaRepositoryImpl(
    private val database: AppDatabase,
    private val mediaDao: MediaDao,
    private val dispatchers: AppDispatchers
) : MediaRepository {

    override fun getTimelineStage1(limitMonths: Int): Flow<List<TimelineMediaProjection>> =
        mediaDao.getTimelineStage1(limitMonths).flowOn(dispatchers.io)

    override fun getTimelineStage2(): Flow<List<TimelineMediaProjection>> =
        mediaDao.getTimelineStage2().flowOn(dispatchers.io)

    override suspend fun getById(id: String): MediaEntity? = mediaDao.getById(id)

    override suspend fun getByCloudId(cloudId: String): MediaEntity? =
        mediaDao.getByCloudId(cloudId)

    override suspend fun getAllByCloudId(cloudId: String): List<MediaEntity> =
        mediaDao.getAllByCloudId(cloudId)

    override suspend fun getAllCloudIds(): List<String> = mediaDao.getAllCloudIds()

    override suspend fun getLocalRows(isVideo: Boolean): List<MediaEntity> =
        mediaDao.getLocalRows(isVideo)

    override suspend fun findBySizeAndStatus(
        sizeBytes: Long,
        syncStatus: SyncStatus
    ): List<MediaEntity> = mediaDao.findBySizeAndStatus(sizeBytes, syncStatus)

    override suspend fun upsert(media: MediaEntity) = mediaDao.upsert(media)

    override suspend fun upsertAll(mediaList: List<MediaEntity>) = mediaDao.upsertAll(mediaList)

    override suspend fun deleteById(id: String) = mediaDao.deleteById(id)

    override suspend fun transitionToLocalOnlyByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        cloudIds.chunked(BATCH_SIZE).forEach { chunk ->
            mediaDao.transitionToLocalOnlyByCloudIds(chunk)
        }
    }

    override suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        cloudIds.chunked(BATCH_SIZE).forEach { chunk ->
            mediaDao.deleteCloudOnlyByCloudIds(chunk)
        }
    }

    override suspend fun deleteByCloudId(cloudId: String) = mediaDao.deleteByCloudId(cloudId)

    override suspend fun deleteByCloudIds(cloudIds: List<String>) {
        if (cloudIds.isEmpty()) return
        cloudIds.chunked(BATCH_SIZE).forEach { chunk ->
            mediaDao.deleteByCloudIds(chunk)
        }
    }

    override suspend fun <T> withTransaction(block: suspend () -> T): T {
        return database.withTransaction(block)
    }

    private companion object {
        const val BATCH_SIZE = 500
    }
}