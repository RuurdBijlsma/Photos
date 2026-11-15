package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.dao.DeviceFolderDao
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import dev.ruurd.photos.domain.repository.DeviceFolderRepository
import kotlinx.coroutines.withContext

class DeviceFolderRepositoryImpl(
    private val deviceFolderDao: DeviceFolderDao,
    private val dispatchers: AppDispatchers
) : DeviceFolderRepository {

    override suspend fun getFolder(bucketId: String): DeviceFolderEntity? = withContext(dispatchers.io) {
        deviceFolderDao.getFolder(bucketId)
    }

    override suspend fun upsertAll(folders: List<DeviceFolderEntity>) = withContext(dispatchers.io) {
        deviceFolderDao.upsertAll(folders)
    }
}
