package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.data.datasource.local.LocalFileOverwriteHelper
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.repository.MediaMutationRepository
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.CancellationException

/**
 * Commits one Rotate tap after the preview already turned optimistically. Persists the new EXIF
 * orientation per sync status and reports the ids the pager has to follow.
 */
class RotateMediaUseCase(
    private val mediaRepository: MediaRepository,
    private val mediaMutationRepository: MediaMutationRepository,
    private val localFileOverwriteHelper: LocalFileOverwriteHelper,
    private val fullResFileCache: FullResFileStore,
) {
    data class Outcome(
        val oldMediaId: String,
        val newMediaId: String,
        val newCloudId: String?,
        /** False when the server rotated but the phone's copy could not be rewritten. */
        val localOverwriteSucceeded: Boolean = true,
    )

    suspend fun commit(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        targetOrientation: Int,
        newRatio: Float,
    ): Result<Outcome> = try {
        val outcome = when (item.syncStatus) {
            SyncStatus.CLOUD_ONLY -> commitCloudOnly(item, details, targetOrientation, newRatio)
            SyncStatus.SYNCED -> commitSynced(item, details, targetOrientation, newRatio)
            SyncStatus.LOCAL_ONLY -> commitLocalOnly(item, targetOrientation, newRatio)
        }
        Result.success(outcome)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }

    /** The server renames the item, so the Room row and the pager move to the new id. */
    private suspend fun commitCloudOnly(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        targetOrientation: Int,
        newRatio: Float,
    ): Outcome {
        val oldCloudId = requireNotNull(item.cloudId) { "Cloud-only item without a cloudId" }
        val newCloudId = mediaMutationRepository
            .updateOrientation(oldCloudId, targetOrientation)
            .getOrThrow()
        val newMediaId = "cloud_$newCloudId"

        mediaRepository.withTransaction {
            val row = mediaRepository.getById(item.id) ?: return@withTransaction
            mediaRepository.deleteById(item.id)
            mediaRepository.upsert(
                row.copy(
                    id = newMediaId,
                    cloudId = newCloudId,
                    ratio = newRatio,
                    hasThumbnails = false,
                ),
            )
        }

        fullResFileCache.evict(oldCloudId)
        // Warm the rotated original before the caller drops the optimistic rotation; a failure
        // here still leaves the Room swap in place.
        warmUpOriginal(newCloudId, details?.fileName)
        return Outcome(item.id, newMediaId, newCloudId)
    }

    /**
     * The row keeps its local id but adopts the new cloudId, and the phone's file is replaced with
     * the rotated original so local bytes, size and hash stay identical to the server.
     */
    private suspend fun commitSynced(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        targetOrientation: Int,
        newRatio: Float,
    ): Outcome {
        val oldCloudId = requireNotNull(item.cloudId) { "Synced item without a cloudId" }
        val newCloudId = mediaMutationRepository
            .updateOrientation(oldCloudId, targetOrientation)
            .getOrThrow()

        mediaRepository.withTransaction {
            val row = mediaRepository.getById(item.id) ?: return@withTransaction
            // A sync racing the PUT may already have inserted the new cloud row; drop it so the
            // same photo is not visible twice.
            mediaRepository.deleteById("cloud_$newCloudId")
            mediaRepository.upsert(
                row.copy(
                    cloudId = newCloudId,
                    ratio = newRatio,
                    hasThumbnails = false,
                    autoBackupSuppressed = false,
                ),
            )
        }

        fullResFileCache.evict(oldCloudId)
        val overwritten = item.localContentUri?.let { uri ->
            localFileOverwriteHelper.overwriteFromCloud(newCloudId, uri, details?.fileName)
        } ?: false
        if (overwritten) {
            details?.sizeBytes?.let { size ->
                mediaRepository.getById(item.id)?.let { row ->
                    mediaRepository.upsert(row.copy(sizeBytes = size))
                }
            }
        }
        return Outcome(item.id, item.id, newCloudId, localOverwriteSucceeded = overwritten)
    }

    /** Rewrites the EXIF orientation in place; `MediaProvider` refreshes size and orientation. */
    private suspend fun commitLocalOnly(
        item: TimelineMediaProjection,
        targetOrientation: Int,
        newRatio: Float,
    ): Outcome {
        val contentUri = requireNotNull(item.localContentUri) {
            "Local-only item without a content uri"
        }
        val newSize = requireNotNull(
            localFileOverwriteHelper.writeOrientation(contentUri, targetOrientation),
        ) { "Could not write the EXIF orientation" }
        mediaRepository.getById(item.id)?.let { row ->
            mediaRepository.upsert(row.copy(ratio = newRatio, sizeBytes = newSize))
        }
        return Outcome(item.id, item.id, null)
    }

    private suspend fun warmUpOriginal(cloudId: String, fileName: String?) {
        try {
            fullResFileCache.get(cloudId, fileName)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // The pager retries the download when it settles on the item.
        }
    }
}
