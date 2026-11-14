package dev.ruurd.photos.data.repository

import androidx.room.withTransaction
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.dao.MediaDao
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.withContext

class MediaRepositoryImpl(
    private val database: AppDatabase,
    private val mediaDao: MediaDao,
    private val dispatchers: AppDispatchers
) : MediaRepository {

    override fun getTimelineStage1(limitMonths: Int): Flow<List<MediaEntity>> =
        mediaDao.getTimelineStage1(limitMonths).flowOn(dispatchers.io)

    override fun getTimelineStage2(): Flow<List<MediaEntity>> =
        mediaDao.getTimelineStage2().flowOn(dispatchers.io)

    override fun countVisible(): Flow<Int> =
        mediaDao.countVisibleFlow().flowOn(dispatchers.io)

    override fun countTotal(): Flow<Int> =
        mediaDao.countTotalFlow().flowOn(dispatchers.io)

    override suspend fun getById(id: String): MediaEntity? = withContext(dispatchers.io) {
        mediaDao.getById(id)
    }

    override suspend fun getByCloudId(cloudId: String): MediaEntity? = withContext(dispatchers.io) {
        mediaDao.getByCloudId(cloudId)
    }

    override suspend fun getAllCloudIds(): List<String> = withContext(dispatchers.io) {
        mediaDao.getAllCloudIds()
    }

    override suspend fun upsert(media: MediaEntity) = withContext(dispatchers.io) {
        mediaDao.upsert(media)
    }

    override suspend fun upsertAll(mediaList: List<MediaEntity>) = withContext(dispatchers.io) {
        mediaDao.upsertAll(mediaList)
    }

    override suspend fun transitionToLocalOnly(cloudId: String, isVisible: Boolean) = withContext(dispatchers.io) {
        mediaDao.transitionToLocalOnly(cloudId, isVisible)
    }

    override suspend fun deleteCloudOnlyByCloudId(cloudId: String) = withContext(dispatchers.io) {
        mediaDao.deleteCloudOnlyByCloudId(cloudId)
    }

    override suspend fun deleteByCloudId(cloudId: String) = withContext(dispatchers.io) {
        mediaDao.deleteByCloudId(cloudId)
    }

    override suspend fun deleteByCloudIds(cloudIds: List<String>) = withContext(dispatchers.io) {
        mediaDao.deleteByCloudIds(cloudIds)
    }

    override suspend fun <T> withTransaction(block: suspend () -> T): T {
        return database.withTransaction(block)
    }
}
