package dev.ruurd.photos.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Update
import androidx.room.Upsert
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import kotlinx.coroutines.flow.Flow

@Dao
interface MediaDao {

    @Query(
        """
        SELECT id, cloud_id, local_content_uri, year_month, ratio, is_video, duration_ms, has_thumbnails, sync_status, sort_timestamp, timezone_offset_minutes
        FROM media_item
        WHERE is_visible_in_timeline = 1
          AND year_month >= COALESCE(
            (
              SELECT MIN(year_month) FROM (
                SELECT DISTINCT year_month FROM media_item
                WHERE is_visible_in_timeline = 1
                ORDER BY year_month DESC
                LIMIT :limitMonths
              )
            ),
            0
          )
        ORDER BY year_month DESC, sort_timestamp DESC, id DESC
        """
    )
    fun getTimelineStage1(limitMonths: Int = 2): Flow<List<TimelineMediaProjection>>

    @Query(
        """
        SELECT id, cloud_id, local_content_uri, year_month, ratio, is_video, duration_ms, has_thumbnails, sync_status, sort_timestamp, timezone_offset_minutes
        FROM media_item
        WHERE is_visible_in_timeline = 1
        ORDER BY year_month DESC, sort_timestamp DESC, id DESC
        """
    )
    fun getTimelineStage2(): Flow<List<TimelineMediaProjection>>

    @Query("SELECT * FROM media_item WHERE id = :id LIMIT 1")
    suspend fun getById(id: String): MediaEntity?

    @Query("SELECT * FROM media_item WHERE cloud_id = :cloudId LIMIT 1")
    suspend fun getByCloudId(cloudId: String): MediaEntity?

    @Query("SELECT * FROM media_item WHERE cloud_id = :cloudId")
    suspend fun getAllByCloudId(cloudId: String): List<MediaEntity>

    @Query("SELECT * FROM media_item WHERE local_content_uri IS NOT NULL AND is_video = :isVideo")
    suspend fun getLocalRows(isVideo: Boolean): List<MediaEntity>

    @Query("SELECT DISTINCT cloud_id FROM media_item WHERE cloud_id IS NOT NULL")
    suspend fun getAllCloudIds(): List<String>

    @Query(
        """
        SELECT * FROM media_item
        WHERE size_bytes = :sizeBytes AND sync_status = :syncStatus
        ORDER BY is_visible_in_timeline DESC, id ASC
        """
    )
    suspend fun findBySizeAndStatus(sizeBytes: Long, syncStatus: SyncStatus): List<MediaEntity>

    @Upsert
    suspend fun upsert(media: MediaEntity)

    @Upsert
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
            is_visible_in_timeline = COALESCE(
                (SELECT f.show_in_timeline FROM device_folder f WHERE f.bucket_id = media_item.bucket_id),
                1
            )
        WHERE cloud_id IN (:cloudIds) AND local_content_uri IS NOT NULL
        """
    )
    suspend fun transitionToLocalOnlyByCloudIds(
        cloudIds: List<String>,
        localOnlyStatus: SyncStatus = SyncStatus.LOCAL_ONLY
    )

    @Query(
        """
        DELETE FROM media_item
        WHERE cloud_id IN (:cloudIds) AND local_content_uri IS NULL
        """
    )
    suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>)

    @Query("DELETE FROM media_item")
    suspend fun clearAll()
}