package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.domain.model.LocalMediaFile
import kotlinx.coroutines.flow.Flow

/** Thin abstraction over the `MediaStore` queries so the scan use case is testable. */
interface LocalMediaStore {
    suspend fun queryImages(modifiedAfterGeneration: Long? = null): List<LocalMediaFile>
    suspend fun queryVideos(modifiedAfterGeneration: Long? = null): List<LocalMediaFile>

    /** Slim URI sets used by incremental scans to detect deletions without reading all columns. */
    suspend fun queryImageUris(): Set<String>
    suspend fun queryVideoUris(): Set<String>

    /** `MediaStore` bumps this whenever the volume's contents change. */
    suspend fun generation(): Long

    /** Changes if `MediaProvider` resets or rebuilds its database (resetting generation numbers). */
    suspend fun version(): String

    /** Emits (debounced) whenever `MediaStore` images or videos change while collected. */
    fun observeChanges(): Flow<Unit>
}