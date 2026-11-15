package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.database.dao.DeviceFolderDao
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import dev.ruurd.photos.domain.repository.DeviceFolderRepository

class DeviceFolderRepositoryImpl(
    private val deviceFolderDao: DeviceFolderDao
) : DeviceFolderRepository {

    override suspend fun getFolder(bucketId: String): DeviceFolderEntity? =
        deviceFolderDao.getFolder(bucketId)

    override suspend fun upsertAll(folders: List<DeviceFolderEntity>) =
        deviceFolderDao.upsertAll(folders)
}