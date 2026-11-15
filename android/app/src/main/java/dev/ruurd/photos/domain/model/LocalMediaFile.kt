package dev.ruurd.photos.domain.model

/** One non-pending, non-trashed MediaStore row. */
data class LocalMediaFile(
    val mediaStoreId: Long,
    val contentUri: String,
    val bucketId: String,
    val bucketName: String?,
    val relativePath: String?,
    val sizeBytes: Long,
    val width: Int,
    val height: Int,
    val orientation: Int,
    val dateTakenMs: Long,
    val mimeType: String?,
    val isVideo: Boolean,
    val durationMs: Int?
)
