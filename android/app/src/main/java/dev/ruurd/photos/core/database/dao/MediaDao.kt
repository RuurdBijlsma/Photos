package dev.ruurd.photos.core.database.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import androidx.room.Update
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import kotlinx.coroutines.flow.Flow

@Dao
interface MediaDao {

    @Query(
        """
        SELECT * FROM media_item
        WHERE is_visible_in_timeline = 1
          AND year_month IN (
            SELECT DISTINCT year_month FROM media_item
            WHERE is_visible_in_timeline = 1
            ORDER BY year_month DESC
            LIMIT :limitMonths
          )
        ORDER BY year_month DESC, sort_timestamp DESC, id DESC
        """
    )
    fun getTimelineStage1(limitMonths: Int = 2): Flow<List<MediaEntity>>

    @Query(
        """
        SELECT * FROM media_item
        WHERE is_visible_in_timeline = 1
        ORDER BY year_month DESC, sort_timestamp DESC, id DESC
        """
    )
    fun getTimelineStage2(): Flow<List<MediaEntity>>

    @Query("SELECT * FROM media_item WHERE id = :id LIMIT 1")
    suspend fun getById(id: String): MediaEntity?

    @Query("SELECT * FROM media_item WHERE cloud_id = :cloudId LIMIT 1")
    suspend fun getByCloudId(cloudId: String): MediaEntity?

    @Query("SELECT * FROM media_item WHERE cloud_id = :cloudId")
    suspend fun getAllByCloudId(cloudId: String): List<MediaEntity>

    @Query("SELECT DISTINCT cloud_id FROM media_item WHERE cloud_id IS NOT NULL")
    suspend fun getAllCloudIds(): List<String>

    @Query("SELECT * FROM media_item WHERE size_bytes = :sizeBytes AND sync_status = :syncStatus")
    suspend fun findBySizeAndStatus(sizeBytes: Long, syncStatus: SyncStatus): List<MediaEntity>

    @Query("SELECT COUNT(*) FROM media_item WHERE is_visible_in_timeline = 1")
    fun countVisibleFlow(): Flow<Int>

    @Query("SELECT COUNT(*) FROM media_item")
    fun countTotalFlow(): Flow<Int>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(media: MediaEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(mediaList: List<MediaEntity>)

    @Update
    suspend fun update(media: MediaEntity)

    @Query("DELETE FROM media_item WHERE id = :id")
    suspend fun deleteById(id: String)

    @Query("DELETE FROM media_item WHERE id IN (:ids)")
    suspend fun deleteByIds(ids: List<String>)

    @Query("DELETE FROM media_item WHERE cloud_id = :cloudId")
    suspend fun deleteByCloudId(cloudId: String)

    @Query("DELETE FROM media_item WHERE cloud_id IN (:cloudIds)")
    suspend fun deleteByCloudIds(cloudIds: List<String>)

    @Query(
        """
        UPDATE media_item
        SET cloud_id = NULL,
            sync_status = :localOnlyStatus,
            auto_backup_suppressed = 1,
            is_visible_in_timeline = :isVisible
        WHERE cloud_id = :cloudId AND local_content_uri IS NOT NULL
        """
    )
    suspend fun transitionToLocalOnly(
        cloudId: String,
        isVisible: Boolean,
        localOnlyStatus: SyncStatus = SyncStatus.LOCAL_ONLY
    )

    @Query(
        """
        DELETE FROM media_item
        WHERE cloud_id = :cloudId AND local_content_uri IS NULL
        """
    )
    suspend fun deleteCloudOnlyByCloudId(cloudId: String)

    @Query("DELETE FROM media_item")
    suspend fun clearAll()
}
