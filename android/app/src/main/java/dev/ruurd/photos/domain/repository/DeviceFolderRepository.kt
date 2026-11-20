package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.core.database.entity.DeviceFolderEntity

interface DeviceFolderRepository {
    suspend fun getFolder(bucketId: String): DeviceFolderEntity?
    suspend fun upsertAll(folders: List<DeviceFolderEntity>)
}
