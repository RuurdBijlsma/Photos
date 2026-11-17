package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.core.database.dao.LocalRowRef
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlinx.coroutines.flow.Flow

interface MediaRepository {
    fun getTimelineStage1(limitMonths: Int = 2): Flow<List<TimelineMediaProjection>>
    fun getTimelineStage2(): Flow<List<TimelineMediaProjection>>

    suspend fun getById(id: String): MediaEntity?
    suspend fun getByIds(ids: List<String>): List<MediaEntity>
    suspend fun getByLocalContentUris(uris: List<String>): List<MediaEntity>
    suspend fun getByCloudId(cloudId: String): MediaEntity?
    suspend fun getAllByCloudId(cloudId: String): List<MediaEntity>
    suspend fun getAllByCloudIds(cloudIds: List<String>): List<MediaEntity>
    suspend fun getLocalRows(isVideo: Boolean): List<MediaEntity>
    suspend fun getLocalRowRefs(isVideo: Boolean): List<LocalRowRef>
    suspend fun findBySizeAndStatus(sizeBytes: Long, syncStatus: SyncStatus): List<MediaEntity>
    suspend fun findBySizesAndStatus(sizeBytes: List<Long>, syncStatus: SyncStatus): List<MediaEntity>
    suspend fun getAllCloudIds(): List<String>

    suspend fun upsert(media: MediaEntity)
    suspend fun upsertAll(mediaList: List<MediaEntity>)
    suspend fun deleteById(id: String)
    suspend fun transitionToLocalOnlyByCloudIds(cloudIds: List<String>)
    suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>)
    suspend fun deleteByCloudId(cloudId: String)
    suspend fun deleteByCloudIds(cloudIds: List<String>)
    suspend fun clearAll()
    suspend fun <T> withTransaction(block: suspend () -> T): T
}