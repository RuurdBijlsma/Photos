package dev.ruurd.photos.core.database.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface DeviceFolderDao {

    @Query("SELECT * FROM device_folder ORDER BY display_name ASC")
    fun getAllFolders(): Flow<List<DeviceFolderEntity>>

    @Query("SELECT * FROM device_folder WHERE bucket_id = :bucketId LIMIT 1")
    suspend fun getFolder(bucketId: String): DeviceFolderEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(folder: DeviceFolderEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(folders: List<DeviceFolderEntity>)

    @Query("UPDATE device_folder SET show_in_timeline = :show WHERE bucket_id = :bucketId")
    suspend fun updateShowInTimeline(bucketId: String, show: Boolean)

    @Query("UPDATE device_folder SET backup_enabled = :enabled WHERE bucket_id = :bucketId")
    suspend fun updateBackupEnabled(bucketId: String, enabled: Boolean)

    @Query("DELETE FROM device_folder WHERE bucket_id = :bucketId")
    suspend fun delete(bucketId: String)
}
