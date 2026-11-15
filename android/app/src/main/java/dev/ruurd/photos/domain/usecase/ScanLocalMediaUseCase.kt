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
 * Reconciles `MediaStore` into Room's `media_item` table: applies device deletions first (only
 * with full per-type permission), registers newly seen folders with their default visibility,
 * upserts changed local rows in batches, and links unmatched files to known cloud rows via
 * Pass 1 zero-hash deduplication.
 */
class ScanLocalMediaUseCase(
    private val localMediaStore: LocalMediaStore,
    private val mediaRepository: MediaRepository,
    private val deviceFolderRepository: DeviceFolderRepository,
    private val dispatchers: AppDispatchers,
    private val zoneProvider: () -> ZoneId = ZoneId::systemDefault
) {
    suspend operator fun invoke(access: MediaAccessState): Result<ScanResult> =
        withContext(dispatchers.io) {
            try {
                if (!access.hasAnyAccess) return@withContext Result.success(ScanResult())

                val zone = zoneProvider()
                val images =
                    if (access.images != MediaAccess.NONE) localMediaStore.queryImages() else emptyList()
                val videos =
                    if (access.videos != MediaAccess.NONE) localMediaStore.queryVideos() else emptyList()

                val result = mediaRepository.withTransaction {
                    val initialImageRows = mediaRepository.getLocalRows(isVideo = false)
                    val initialVideoRows = mediaRepository.getLocalRows(isVideo = true)

                    // Reconcile missing local files first so moved/renamed SYNCED files revert
                    // to CLOUD_ONLY before Pass 1 links the replacement MediaStore URI.
                    val removed =
                        reconcileRemovals(access.isFull(images = true), images, initialImageRows) +
                                reconcileRemovals(access.isFull(images = false), videos, initialVideoRows)

                    val currentLocalRows = if (removed > 0) {
                        mediaRepository.getLocalRows(isVideo = false) +
                                mediaRepository.getLocalRows(isVideo = true)
                    } else {
                        initialImageRows + initialVideoRows
                    }

                    val knownByUri = HashMap<String, MediaEntity>(currentLocalRows.size)
                    currentLocalRows.forEach { row ->
                        row.localContentUri?.let { knownByUri[it] = row }
                    }

                    val folders = mutableMapOf<String, DeviceFolderEntity>()
                    val newFolders = mutableListOf<DeviceFolderEntity>()
                    val toUpsert = mutableListOf<MediaEntity>()
                    val matchedCloudRowIds = HashSet<String>()
                    var deduplicated = 0

                    for (file in images + videos) {
                        val folder = folderFor(file, folders, newFolders)
                        val existing = knownByUri[file.contentUri]
                        if (collectUpsertForFile(
                                file = file,
                                existing = existing,
                                folder = folder,
                                zone = zone,
                                matchedCloudRowIds = matchedCloudRowIds,
                                toUpsert = toUpsert
                            )
                        ) {
                            deduplicated++
                        }
                    }

                    if (newFolders.isNotEmpty()) {
                        deviceFolderRepository.upsertAll(newFolders)
                    }
                    if (toUpsert.isNotEmpty()) {
                        toUpsert.chunked(500).forEach { chunk ->
                            mediaRepository.upsertAll(chunk)
                        }
                    }

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
        cache: MutableMap<String, DeviceFolderEntity>,
        newFolders: MutableList<DeviceFolderEntity>
    ): DeviceFolderEntity {
        cache[file.bucketId]?.let { return it }

        val folder = deviceFolderRepository.getFolder(file.bucketId) ?: run {
            val defaults = DeviceFolderRules.defaultsFor(file.relativePath)
            val displayName = file.bucketName?.takeIf { it.isNotBlank() }
                ?: file.relativePath?.trimEnd('/')?.takeIf { it.isNotEmpty() }
                ?: file.bucketId

            DeviceFolderEntity(
                bucketId = file.bucketId,
                displayName = displayName,
                relativePath = file.relativePath ?: "",
                showInTimeline = defaults.showInTimeline,
                backupEnabled = defaults.backupEnabled
            ).also { newFolders.add(it) }
        }

        cache[file.bucketId] = folder
        return folder
    }

    /** @return true when the local file was linked to an existing cloud-only row. */
    private suspend fun collectUpsertForFile(
        file: LocalMediaFile,
        existing: MediaEntity?,
        folder: DeviceFolderEntity,
        zone: ZoneId,
        matchedCloudRowIds: MutableSet<String>,
        toUpsert: MutableList<MediaEntity>
    ): Boolean {
        val local = LocalMediaMapper.toEntity(file, folder.showInTimeline, zone)
        if (existing == null) {
            return linkToCloudOrQueueInsert(local, matchedCloudRowIds, toUpsert)
        }

        // A cloud-linked row keeps the server's authoritative media metadata and visibility;
        // the local scan only refreshes folder details.
        val linked = existing.cloudId != null
        val updated = existing.copy(
            bucketId = local.bucketId,
            bucketName = local.bucketName,
            relativePath = local.relativePath,
            isVisibleInTimeline = if (linked) existing.isVisibleInTimeline else folder.showInTimeline,
            sizeBytes = if (linked) existing.sizeBytes else local.sizeBytes,
            ratio = if (linked) existing.ratio else local.ratio,
            sortTimestamp = if (linked) existing.sortTimestamp else local.sortTimestamp,
            yearMonth = if (linked) existing.yearMonth else local.yearMonth,
            durationMs = if (linked) existing.durationMs else local.durationMs
        )
        if (updated != existing) {
            toUpsert.add(updated)
        }
        return false
    }

    /**
     * Pass 1: same byte size, same aspect ratio, and a timestamp that differs only by a timezone
     * offset means it is the same media. Zero hashing, zero disk reads.
     */
    private suspend fun linkToCloudOrQueueInsert(
        local: MediaEntity,
        matchedCloudRowIds: MutableSet<String>,
        toUpsert: MutableList<MediaEntity>
    ): Boolean {
        val match = mediaRepository.findBySizeAndStatus(local.sizeBytes, SyncStatus.CLOUD_ONLY)
            .firstOrNull { candidate ->
                candidate.id !in matchedCloudRowIds &&
                        PassOneDeduplication.isMatch(
                            cloudRatio = candidate.ratio,
                            cloudSortTimestamp = candidate.sortTimestamp,
                            localRatio = local.ratio,
                            localTimestampMs = local.sortTimestamp
                        )
            }

        if (match == null) {
            toUpsert.add(local)
            return false
        }

        matchedCloudRowIds.add(match.id)
        toUpsert.add(
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
                    bucketId = null,
                    bucketName = null,
                    relativePath = null,
                    syncStatus = SyncStatus.CLOUD_ONLY,
                    isVisibleInTimeline = true
                )
            )
        }
    }
}