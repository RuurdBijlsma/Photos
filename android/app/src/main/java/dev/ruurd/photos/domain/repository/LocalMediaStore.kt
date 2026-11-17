package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.domain.model.LocalMediaFile

/** Thin abstraction over the `MediaStore` queries so the scan use case is testable. */
interface LocalMediaStore {
    suspend fun queryImages(): List<LocalMediaFile>
    suspend fun queryVideos(): List<LocalMediaFile>

    /** `MediaStore` bumps this whenever the volume's contents change. */
    suspend fun generation(): Long
}
