package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.data.datasource.local.LocalMediaMapper
import dev.ruurd.photos.domain.model.DeviceFolderRules
import dev.ruurd.photos.domain.model.LocalMediaFile
import dev.ruurd.photos.domain.model.MediaAccess
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.model.PassOneDeduplication
import dev.ruurd.photos.domain.model.ScanGuards
import dev.ruurd.photos.domain.repository.DeviceFolderRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.withContext
import java.time.ZoneId

data class ScanResult(
    val scanned: Int = 0,
    val deduplicated: Int = 0,
    val removed: Int = 0
)

/**
 * Reconciles `MediaStore` into Room's `media_item` table: registers newly seen folders with their
 * default visibility, upserts local rows, links them to already-known cloud rows via Pass 1
 * zero-hash deduplication, and (only with full per-type permission) applies device deletions.
 */
class ScanLocalMediaUseCase(
    private val localMediaStore: LocalMediaStore,
    private val mediaRepository: MediaRepository,
    private val deviceFolderRepository: DeviceFolderRepository,
    private val dispatchers: AppDispatchers,
    private val zone: ZoneId = ZoneId.systemDefault()
) {
    suspend operator fun invoke(access: MediaAccessState): Result<ScanResult> = withContext(dispatchers.io) {
        try {
            if (!access.hasAnyAccess) return@withContext Result.success(ScanResult())

            val images = if (access.images != MediaAccess.NONE) localMediaStore.queryImages() else emptyList()
            val videos = if (access.videos != MediaAccess.NONE) localMediaStore.queryVideos() else emptyList()

            val result = mediaRepository.withTransaction {
                val imageRows = mediaRepository.getLocalRows(isVideo = false)
                val videoRows = mediaRepository.getLocalRows(isVideo = true)
                val knownByUri = HashMap<String, MediaEntity>(imageRows.size + videoRows.size)
                (imageRows + videoRows).forEach { row ->
                    row.localContentUri?.let { knownByUri[it] = row }
                }

                val folders = mutableMapOf<String, DeviceFolderEntity>()
                var deduplicated = 0
                for (file in images + videos) {
                    if (upsertLocalFile(file, knownByUri[file.contentUri], folderFor(file, folders))) {
                        deduplicated++
                    }
                }

                val removed = reconcileRemovals(access.isFull(images = true), images, imageRows) +
                    reconcileRemovals(access.isFull(images = false), videos, videoRows)

                ScanResult(
                    scanned = images.size + videos.size,
                    deduplicated = deduplicated,
                    removed = removed
                )
            }

            Result.success(result)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private suspend fun folderFor(
        file: LocalMediaFile,
        cache: MutableMap<String, DeviceFolderEntity>
    ): DeviceFolderEntity {
        cache[file.bucketId]?.let { return it }

        val folder = deviceFolderRepository.getFolder(file.bucketId) ?: run {
            val defaults = DeviceFolderRules.defaultsFor(file.relativePath)
            DeviceFolderEntity(
                bucketId = file.bucketId,
                displayName = file.bucketName ?: file.relativePath ?: file.bucketId,
                relativePath = file.relativePath ?: "",
                showInTimeline = defaults.showInTimeline,
                backupEnabled = defaults.backupEnabled
            ).also { deviceFolderRepository.upsertAll(listOf(it)) }
        }

        cache[file.bucketId] = folder
        return folder
    }

    /** @return true when the local file was linked to an existing cloud-only row. */
    private suspend fun upsertLocalFile(
        file: LocalMediaFile,
        existing: MediaEntity?,
        folder: DeviceFolderEntity
    ): Boolean {
        val local = LocalMediaMapper.toEntity(file, folder.showInTimeline, zone)
        if (existing == null) return linkToCloudOrInsert(local)

        // A cloud-linked row keeps the server's timestamp, month and visibility; the scan only
        // refreshes folder details. A local row follows its folder and the file MediaStore reports.
        val linked = existing.cloudId != null
        mediaRepository.upsert(
            existing.copy(
                bucketId = local.bucketId,
                bucketName = local.bucketName,
                relativePath = local.relativePath,
                isVisibleInTimeline = if (linked) {
                    existing.isVisibleInTimeline
                } else {
                    folder.showInTimeline
                },
                sizeBytes = local.sizeBytes,
                ratio = local.ratio,
                sortTimestamp = if (linked) existing.sortTimestamp else local.sortTimestamp,
                yearMonth = if (linked) existing.yearMonth else local.yearMonth,
                durationMs = local.durationMs
            )
        )
        return false
    }

    /**
     * Pass 1: same byte size, same aspect ratio, and a timestamp that differs only by a timezone
     * offset means it is the same media. Zero hashing, zero disk reads.
     */
    private suspend fun linkToCloudOrInsert(local: MediaEntity): Boolean {
        val match = mediaRepository.findBySizeAndStatus(local.sizeBytes, SyncStatus.CLOUD_ONLY)
            .firstOrNull {
                PassOneDeduplication.isMatch(
                    cloudRatio = it.ratio,
                    cloudSortTimestamp = it.sortTimestamp,
                    localRatio = local.ratio,
                    localTimestampMs = local.sortTimestamp
                )
            }

        if (match == null) {
            mediaRepository.upsert(local)
            return false
        }

        // Delete first: `local_content_uri` is unique, so the row cannot exist twice.
        mediaRepository.deleteById(local.id)
        mediaRepository.upsert(
            match.copy(
                localContentUri = local.localContentUri,
                bucketId = local.bucketId,
                bucketName = local.bucketName,
                relativePath = local.relativePath,
                syncStatus = SyncStatus.SYNCED,
                isVisibleInTimeline = true
            )
        )
        return true
    }

    private suspend fun reconcileRemovals(
        fullAccess: Boolean,
        scanned: List<LocalMediaFile>,
        localRows: List<MediaEntity>
    ): Int {
        if (!ScanGuards.shouldReconcileRemovals(fullAccess, scanned.size, localRows.size)) return 0

        val scannedUris = scanned.mapTo(HashSet(scanned.size)) { it.contentUri }
        var removed = 0
        for (row in localRows) {
            if (row.localContentUri in scannedUris) continue
            removeLocalFile(row)
            removed++
        }
        return removed
    }

    private suspend fun removeLocalFile(row: MediaEntity) {
        val siblings = row.cloudId?.let { mediaRepository.getAllByCloudId(it) }.orEmpty()
        val otherCopy = siblings.firstOrNull { it.id != row.id }

        when {
            row.cloudId == null -> mediaRepository.deleteById(row.id)

            otherCopy != null -> {
                mediaRepository.deleteById(row.id)
                // The canonical visible row moved; keep exactly one visible row per cloudId.
                if (row.isVisibleInTimeline && !otherCopy.isVisibleInTimeline) {
                    mediaRepository.upsert(otherCopy.copy(isVisibleInTimeline = true))
                }
            }

            else -> mediaRepository.upsert(
                row.copy(
                    localContentUri = null,
                    syncStatus = SyncStatus.CLOUD_ONLY,
                    isVisibleInTimeline = true
                )
            )
        }
    }
}
