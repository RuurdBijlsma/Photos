package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.dao.LocalRowRef
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
import dev.ruurd.photos.domain.repository.ScanStateStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.withContext
import java.time.ZoneId

data class ScanResult(
    val scanned: Int = 0,
    val deduplicated: Int = 0,
    val removed: Int = 0,
)

/**
 * Reconciles `MediaStore` into Room's `media_item` table: applies device deletions first (only
 * with full per-type permission), registers newly seen folders with their default visibility,
 * upserts changed local rows in batches, and links unmatched files to known cloud rows via
 * Pass 1 zero-hash deduplication.
 *
 * Uses `(AppDatabase.VERSION, MediaStore.getVersion, access, zone, generation)` to skip unchanged
 * scans completely, or query only rows with `GENERATION_MODIFIED > lastGeneration` plus a slim
 * URI diff for deletions when only recent captures/edits occurred.
 */
class ScanLocalMediaUseCase(
    private val localMediaStore: LocalMediaStore,
    private val mediaRepository: MediaRepository,
    private val deviceFolderRepository: DeviceFolderRepository,
    private val scanStateStore: ScanStateStore,
    private val dispatchers: AppDispatchers,
    private val zoneProvider: () -> ZoneId = ZoneId::systemDefault,
) {
    suspend operator fun invoke(access: MediaAccessState): Result<ScanResult> = withContext(dispatchers.io) {
        try {
            if (!access.hasAnyAccess) return@withContext Result.success(ScanResult())

            val zone = zoneProvider()
            if (access.hasPartialAccess) {
                // Partial selection isn't tracked by volume generation; always run full scan.
                return@withContext Result.success(runFullScan(access, zone))
            }

            // Read generation & version *before* querying so any concurrent MediaStore write
            // during the query bumps generation above what we persist, triggering a rescan.
            val currentGeneration = localMediaStore.generation()
            val currentVersion = localMediaStore.version()
            val previous = ScanWatermark.parse(scanStateStore.lastScanSignature())
            val currentWatermark = ScanWatermark(
                dbVersion = AppDatabase.VERSION,
                mediaStoreVersion = currentVersion,
                imageAccess = access.images,
                videoAccess = access.videos,
                zoneId = zone.id,
                generation = currentGeneration,
            )

            val result = when {
                previous != null && previous.matchesContext(currentWatermark) -> {
                    when {
                        currentGeneration == previous.generation -> return@withContext Result.success(ScanResult())

                        currentGeneration > previous.generation ->
                            runIncrementalScan(access, zone, sinceGeneration = previous.generation)

                        else -> runFullScan(access, zone)
                    }
                }

                else -> runFullScan(access, zone)
            }

            scanStateStore.setLastScanSignature(currentWatermark.serialize())
            Result.success(result)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private suspend fun runFullScan(access: MediaAccessState, zone: ZoneId): ScanResult {
        val images =
            if (access.images != MediaAccess.NONE) localMediaStore.queryImages() else emptyList()
        val videos =
            if (access.videos != MediaAccess.NONE) localMediaStore.queryVideos() else emptyList()

        return mediaRepository.withTransaction {
            val initialImageRows = mediaRepository.getLocalRows(isVideo = false)
            val initialVideoRows = mediaRepository.getLocalRows(isVideo = true)

            // Reconcile missing local files first so moved/renamed SYNCED files revert
            // to CLOUD_ONLY before Pass 1 links the replacement MediaStore URI.
            val removed =
                reconcileRemovalsFromFiles(access.isFull(images = true), images, initialImageRows) +
                        reconcileRemovalsFromFiles(access.isFull(images = false), videos, initialVideoRows)

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

            applyFiles(
                files = images + videos,
                knownByUri = knownByUri,
                zone = zone,
                removed = removed,
            )
        }
    }

    private suspend fun runIncrementalScan(
        access: MediaAccessState,
        zone: ZoneId,
        sinceGeneration: Long,
    ): ScanResult {
        val modifiedImages = if (access.images != MediaAccess.NONE) {
            localMediaStore.queryImages(modifiedAfterGeneration = sinceGeneration)
        } else {
            emptyList()
        }
        val modifiedVideos = if (access.videos != MediaAccess.NONE) {
            localMediaStore.queryVideos(modifiedAfterGeneration = sinceGeneration)
        } else {
            emptyList()
        }

        val missingIds = ArrayList<String>()
        var liveImageUris: Set<String>? = null
        var liveVideoUris: Set<String>? = null

        if (access.isFull(images = true)) {
            val imageRefs = mediaRepository.getLocalRowRefs(isVideo = false)
            if (imageRefs.isNotEmpty() || modifiedImages.isNotEmpty()) {
                val liveUris = localMediaStore.queryImageUris()
                liveImageUris = liveUris
                if (imageRefs.isNotEmpty()) {
                    collectMissingIds(imageRefs, liveUris, missingIds)
                }
            }
        }
        if (access.isFull(images = false)) {
            val videoRefs = mediaRepository.getLocalRowRefs(isVideo = true)
            if (videoRefs.isNotEmpty() || modifiedVideos.isNotEmpty()) {
                val liveUris = localMediaStore.queryVideoUris()
                liveVideoUris = liveUris
                if (videoRefs.isNotEmpty()) {
                    collectMissingIds(videoRefs, liveUris, missingIds)
                }
            }
        }

        // Filter out items that were deleted or reverted to IS_PENDING between the delta query
        // and the live URI check so they are not inserted as ghost rows.
        val filteredImages = if (liveImageUris != null && modifiedImages.isNotEmpty()) {
            modifiedImages.filter { it.contentUri in liveImageUris }
        } else {
            modifiedImages
        }
        val filteredVideos = if (liveVideoUris != null && modifiedVideos.isNotEmpty()) {
            modifiedVideos.filter { it.contentUri in liveVideoUris }
        } else {
            modifiedVideos
        }

        val modifiedFiles = filteredImages + filteredVideos
        if (modifiedFiles.isEmpty() && missingIds.isEmpty()) {
            // Generation bumped for something outside our visible scope (e.g. IS_PENDING = 1).
            return ScanResult()
        }

        return mediaRepository.withTransaction {
            var removed = 0
            if (missingIds.isNotEmpty()) {
                val missingRows = mediaRepository.getByIds(missingIds)
                for (row in missingRows) {
                    removeLocalFile(row)
                    removed++
                }
            }

            val knownByUri = if (modifiedFiles.isNotEmpty()) {
                mediaRepository.getByLocalContentUris(modifiedFiles.map { it.contentUri })
                    .mapNotNull { row -> row.localContentUri?.let { it to row } }
                    .toMap()
            } else {
                emptyMap()
            }

            applyFiles(
                files = modifiedFiles,
                knownByUri = knownByUri,
                zone = zone,
                removed = removed,
            )
        }
    }

    private suspend fun applyFiles(
        files: List<LocalMediaFile>,
        knownByUri: Map<String, MediaEntity>,
        zone: ZoneId,
        removed: Int,
    ): ScanResult {
        if (files.isEmpty()) {
            return ScanResult(scanned = 0, deduplicated = 0, removed = removed)
        }

        val folders = mutableMapOf<String, DeviceFolderEntity>()
        val newFolders = mutableListOf<DeviceFolderEntity>()
        val visibleFiles = ArrayList<Pair<LocalMediaFile, DeviceFolderEntity>>(files.size)
        val hiddenFiles = ArrayList<Pair<LocalMediaFile, DeviceFolderEntity>>()

        for (file in files) {
            val folder = folderFor(file, folders, newFolders)
            if (folder.showInTimeline) {
                visibleFiles.add(file to folder)
            } else {
                hiddenFiles.add(file to folder)
            }
        }

        // Process timeline-visible folders before hidden ones so Pass 1 links
        // e.g. DCIM/Camera copies to CLOUD_ONLY rows ahead of Download duplicates.
        val orderedFiles = visibleFiles.apply { addAll(hiddenFiles) }

        val toUpsert = mutableListOf<MediaEntity>()
        val matchedCloudRowIds = HashSet<String>()
        var deduplicated = 0

        for ((file, folder) in orderedFiles) {
            val existing = knownByUri[file.contentUri]
            if (collectUpsertForFile(
                    file = file,
                    existing = existing,
                    folder = folder,
                    zone = zone,
                    matchedCloudRowIds = matchedCloudRowIds,
                    toUpsert = toUpsert,
                )
            ) {
                deduplicated++
            }
        }

        if (newFolders.isNotEmpty()) {
            deviceFolderRepository.upsertAll(newFolders)
        }
        if (toUpsert.isNotEmpty()) {
            mediaRepository.upsertAll(toUpsert)
        }

        return ScanResult(
            scanned = files.size,
            deduplicated = deduplicated,
            removed = removed,
        )
    }

    private fun collectMissingIds(
        localRefs: List<LocalRowRef>,
        liveUris: Set<String>,
        outMissingIds: MutableList<String>,
    ) {
        if (!ScanGuards.shouldReconcileRemovals(true, liveUris.size, localRefs.size)) return
        for (ref in localRefs) {
            if (ref.localContentUri !in liveUris) {
                outMissingIds.add(ref.id)
            }
        }
    }

    private suspend fun folderFor(
        file: LocalMediaFile,
        cache: MutableMap<String, DeviceFolderEntity>,
        newFolders: MutableList<DeviceFolderEntity>,
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
                backupEnabled = defaults.backupEnabled,
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
        toUpsert: MutableList<MediaEntity>,
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
            timezoneOffsetMinutes = if (linked) existing.timezoneOffsetMinutes else local.timezoneOffsetMinutes,
            yearMonth = if (linked) existing.yearMonth else local.yearMonth,
            durationMs = if (linked) existing.durationMs else local.durationMs,
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
        toUpsert: MutableList<MediaEntity>,
    ): Boolean {
        val candidates = mediaRepository.findBySizesAndStatus(listOf(local.sizeBytes), SyncStatus.CLOUD_ONLY)
        val match = PassOneDeduplication.findMatch(
            candidates = candidates,
            excludeIds = matchedCloudRowIds,
            ratio = local.ratio,
            sortTimestamp = local.sortTimestamp,
        )

        if (match == null) {
            toUpsert.add(local)
            return false
        }

        matchedCloudRowIds.add(match.id)
        toUpsert.add(PassOneDeduplication.mergeSynced(local = local, cloud = match, id = match.id))
        return true
    }

    private suspend fun reconcileRemovalsFromFiles(
        fullAccess: Boolean,
        scanned: List<LocalMediaFile>,
        localRows: List<MediaEntity>,
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
                    isVisibleInTimeline = true,
                ),
            )
        }
    }

    internal data class ScanWatermark(
        val dbVersion: Int,
        val mediaStoreVersion: String,
        val imageAccess: MediaAccess,
        val videoAccess: MediaAccess,
        val zoneId: String,
        val generation: Long,
    ) {
        fun matchesContext(other: ScanWatermark): Boolean = dbVersion == other.dbVersion &&
                mediaStoreVersion == other.mediaStoreVersion &&
                imageAccess == other.imageAccess &&
                videoAccess == other.videoAccess &&
                zoneId == other.zoneId

        fun serialize(): String = "$dbVersion|$mediaStoreVersion|$imageAccess|$videoAccess|$zoneId|$generation"

        companion object {
            fun parse(raw: String?): ScanWatermark? {
                if (raw.isNullOrBlank()) return null
                val parts = raw.split('|')
                if (parts.size != 6) return null
                val dbVersion = parts[0].toIntOrNull() ?: return null
                val imageAccess = runCatching { MediaAccess.valueOf(parts[2]) }.getOrNull() ?: return null
                val videoAccess = runCatching { MediaAccess.valueOf(parts[3]) }.getOrNull() ?: return null
                val generation = parts[5].toLongOrNull() ?: return null
                return ScanWatermark(
                    dbVersion = dbVersion,
                    mediaStoreVersion = parts[1],
                    imageAccess = imageAccess,
                    videoAccess = videoAccess,
                    zoneId = parts[4],
                    generation = generation,
                )
            }
        }
    }
}