package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.core.database.entity.MediaEntity
import kotlinx.coroutines.flow.Flow

interface MediaRepository {
    fun getTimelineStage1(limitMonths: Int = 2): Flow<List<MediaEntity>>
    fun getTimelineStage2(): Flow<List<MediaEntity>>
    fun countVisible(): Flow<Int>
    fun countTotal(): Flow<Int>

    suspend fun getById(id: String): MediaEntity?
    suspend fun getByCloudId(cloudId: String): MediaEntity?
    suspend fun getAllCloudIds(): List<String>

    suspend fun upsert(media: MediaEntity)
    suspend fun upsertAll(mediaList: List<MediaEntity>)
    suspend fun transitionToLocalOnly(cloudId: String, isVisible: Boolean)
    suspend fun deleteCloudOnlyByCloudId(cloudId: String)
    suspend fun deleteByCloudId(cloudId: String)
    suspend fun deleteByCloudIds(cloudIds: List<String>)
    suspend fun <T> withTransaction(block: suspend () -> T): T
}
