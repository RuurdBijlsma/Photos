package dev.ruurd.photos.core.database

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import dev.ruurd.photos.core.database.dao.DeviceFolderDao
import dev.ruurd.photos.core.database.dao.MediaDao
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import dev.ruurd.photos.core.database.entity.MediaEntity

@Database(
    entities = [
        MediaEntity::class,
        DeviceFolderEntity::class,
    ],
    version = AppDatabase.VERSION,
    exportSchema = false,
)
abstract class AppDatabase : RoomDatabase() {

    abstract fun mediaDao(): MediaDao
    abstract fun deviceFolderDao(): DeviceFolderDao

    companion object {
        /** Bumping this drops every table, so callers caching derived state include it. */
        const val VERSION = 2

        private const val DATABASE_NAME = "photos.db"

        fun create(context: Context): AppDatabase = Room.databaseBuilder(
            context.applicationContext,
            AppDatabase::class.java,
            DATABASE_NAME,
        )
            .setJournalMode(JournalMode.WRITE_AHEAD_LOGGING)
            .fallbackToDestructiveMigration(dropAllTables = true)
            .build()
    }
}
