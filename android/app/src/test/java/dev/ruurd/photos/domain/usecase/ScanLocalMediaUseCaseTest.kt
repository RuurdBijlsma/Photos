package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.entity.DeviceFolderEntity
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.domain.model.LocalMediaFile
import dev.ruurd.photos.domain.model.MediaAccess
import dev.ruurd.photos.domain.model.MediaAccessState
import dev.ruurd.photos.domain.repository.DeviceFolderRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
import dev.ruurd.photos.domain.repository.ScanStateStore
import dev.ruurd.photos.testing.FakeMediaRepository
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.ZoneId

@OptIn(ExperimentalCoroutinesApi::class)
class ScanLocalMediaUseCaseTest {

    private val testDispatcher = StandardTestDispatcher()

    private val dispatchers = object : AppDispatchers {
        override val default: CoroutineDispatcher = testDispatcher
        override val io: CoroutineDispatcher = testDispatcher
        override val main: CoroutineDispatcher = testDispatcher
    }

    private class FakeLocalMediaStore(
        var images: List<Pair<Long, LocalMediaFile>> = emptyList(),
        var videos: List<Pair<Long, LocalMediaFile>> = emptyList(),
        var generation: Long = 1L,
        var version: String = "v1"
    ) : LocalMediaStore {
        val lastModifiedAfterRequests = mutableListOf<Long?>()

        fun setImageFiles(files: List<LocalMediaFile>, itemGeneration: Long = generation) {
            images = files.map { itemGeneration to it }
        }

        override suspend fun queryImages(modifiedAfterGeneration: Long?): List<LocalMediaFile> {
            lastModifiedAfterRequests.add(modifiedAfterGeneration)
            return if (modifiedAfterGeneration == null) {
                images.map { it.second }
            } else {
                images.filter { it.first > modifiedAfterGeneration }.map { it.second }
            }
        }

        override suspend fun queryVideos(modifiedAfterGeneration: Long?): List<LocalMediaFile> {
            return if (modifiedAfterGeneration == null) {
                videos.map { it.second }
            } else {
                videos.filter { it.first > modifiedAfterGeneration }.map { it.second }
            }
        }

        override suspend fun queryImageUris(): Set<String> =
            images.mapTo(HashSet()) { it.second.contentUri }

        override suspend fun queryVideoUris(): Set<String> =
            videos.mapTo(HashSet()) { it.second.contentUri }

        override suspend fun generation(): Long = generation

        override suspend fun version(): String = version

        override fun observeChanges(): Flow<Unit> = emptyFlow()
    }

    private class FakeScanStateStore : ScanStateStore {
        var signature: String? = null
        override val hasScanSignature = MutableStateFlow(false)

        override suspend fun lastScanSignature(): String? = signature
        override suspend fun setLastScanSignature(signature: String) {
            this.signature = signature
            hasScanSignature.value = true
        }
    }

    private class FakeDeviceFolderRepository : DeviceFolderRepository {
        val folders = mutableMapOf<String, DeviceFolderEntity>()

        override suspend fun getFolder(bucketId: String): DeviceFolderEntity? = folders[bucketId]

        override suspend fun upsertAll(folders: List<DeviceFolderEntity>) {
            folders.forEach { this.folders[it.bucketId] = it }
        }
    }

    private fun file(
        id: Long,
        relativePath: String = "DCIM/Camera/",
        bucketId: String = "camera",
        width: Int = 4000,
        height: Int = 3000,
        dateTakenMs: Long = 1_730_000_000_000L,
        isVideo: Boolean = false
    ) = LocalMediaFile(
        mediaStoreId = id,
        contentUri = "content://media/external/media/$id",
        bucketId = bucketId,
        bucketName = relativePath.trimEnd('/').substringAfterLast('/'),
        relativePath = relativePath,
        sizeBytes = 1_000L + id,
        width = width,
        height = height,
        orientation = 0,
        dateTakenMs = dateTakenMs,
        mimeType = if (isVideo) "video/mp4" else "image/jpeg",
        isVideo = isVideo,
        durationMs = if (isVideo) 5_000 else null
    )

    private fun useCase(
        store: FakeLocalMediaStore,
        repo: FakeMediaRepository,
        folders: FakeDeviceFolderRepository,
        scanState: FakeScanStateStore = FakeScanStateStore(),
        zoneProvider: () -> ZoneId = { ZoneId.of("UTC") }
    ) = ScanLocalMediaUseCase(
        localMediaStore = store,
        mediaRepository = repo,
        deviceFolderRepository = folders,
        scanStateStore = scanState,
        dispatchers = dispatchers,
        zoneProvider = zoneProvider
    )

    private fun cloudOnly(
        cloudId: String,
        sizeBytes: Long,
        ratio: Float = 4000f / 3000f,
        sortTimestamp: Long = 1_730_000_000_000L
    ) = MediaEntity(
        id = "cloud_$cloudId",
        cloudId = cloudId,
        sizeBytes = sizeBytes,
        ratio = ratio,
        sortTimestamp = sortTimestamp,
        syncStatus = SyncStatus.CLOUD_ONLY
    )

    @Test
    fun `camera folder is registered and its files become visible local rows`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            val folders = FakeDeviceFolderRepository()
            val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

            val result = useCase(store, repo, folders)(fullAccess()).getOrThrow()

            assertEquals(1, result.scanned)
            assertEquals(0, result.deduplicated)

            val folder = folders.folders.getValue("camera")
            assertTrue(folder.showInTimeline)
            assertTrue(folder.backupEnabled)

            val row = repo.items.getValue("local_1")
            assertEquals(SyncStatus.LOCAL_ONLY, row.syncStatus)
            assertEquals("camera", row.bucketId)
            assertTrue(row.isVisibleInTimeline)
            assertEquals(1_730_000_000_000L, row.sortTimestamp)
        }

    @Test
    fun `screenshot folder is registered but hidden from the timeline`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val folders = FakeDeviceFolderRepository()
        val store = FakeLocalMediaStore().apply {
            setImageFiles(listOf(file(2, relativePath = "DCIM/Screenshots/", bucketId = "shots")))
        }

        useCase(store, repo, folders)(fullAccess()).getOrThrow()

        assertFalse(folders.folders.getValue("shots").showInTimeline)
        assertFalse(repo.items.getValue("local_2").isVisibleInTimeline)
    }

    @Test
    fun `existing folder flags are not overwritten by rediscovery`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val folders = FakeDeviceFolderRepository().apply {
            folders["camera"] = DeviceFolderEntity(
                bucketId = "camera",
                displayName = "Camera",
                relativePath = "DCIM/Camera/",
                showInTimeline = false,
                backupEnabled = false
            )
        }
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(3))) }

        useCase(store, repo, folders)(fullAccess()).getOrThrow()

        assertFalse(folders.folders.getValue("camera").showInTimeline)
        assertFalse(repo.items.getValue("local_3").isVisibleInTimeline)
    }

    @Test
    fun `pass 1 links a matching cloud row without hashing`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(cloudOnly("a1B2c3D4e5", sizeBytes = 1_001L))
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        assertEquals(1, result.deduplicated)
        assertNull(repo.getById("local_1"))

        val synced = repo.getByCloudId("a1B2c3D4e5")!!
        assertEquals(SyncStatus.SYNCED, synced.syncStatus)
        assertEquals("content://media/external/media/1", synced.localContentUri)
        assertEquals("camera", synced.bucketId)
        assertTrue(synced.isVisibleInTimeline)
    }

    @Test
    fun `pass 1 prefers visible camera copy over hidden duplicate folder`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.upsert(cloudOnly("c1", sizeBytes = 1_001L))
            val hiddenCopy = file(1, relativePath = "Download/", bucketId = "downloads")
                .copy(sizeBytes = 1_001L)
            val cameraCopy = file(2, relativePath = "DCIM/Camera/", bucketId = "camera")
                .copy(sizeBytes = 1_001L)
            // Download appears before Camera in the cursor
            val store = FakeLocalMediaStore().apply {
                setImageFiles(listOf(hiddenCopy, cameraCopy))
            }

            val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

            assertEquals(1, result.deduplicated)

            val synced = repo.getByCloudId("c1")!!
            assertEquals(SyncStatus.SYNCED, synced.syncStatus)
            assertEquals("content://media/external/media/2", synced.localContentUri)
            assertEquals("camera", synced.bucketId)
            assertTrue(synced.isVisibleInTimeline)

            val hiddenRow = repo.getById("local_1")!!
            assertEquals(SyncStatus.LOCAL_ONLY, hiddenRow.syncStatus)
            assertFalse(hiddenRow.isVisibleInTimeline)
        }

    @Test
    fun `pass 1 tolerates a timezone-offset timestamp`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            cloudOnly("tz", sizeBytes = 1_001L, sortTimestamp = 1_730_000_000_000L + 2 * 3_600_000L)
        )
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        assertEquals(1, result.deduplicated)
        assertEquals(SyncStatus.SYNCED, repo.getByCloudId("tz")!!.syncStatus)
    }

    @Test
    fun `pass 1 ignores a size collision with a different aspect ratio`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(cloudOnly("other", sizeBytes = 1_001L, ratio = 0.75f))
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        assertEquals(0, result.deduplicated)
        assertNotNull(repo.getById("local_1"))
        assertEquals(SyncStatus.CLOUD_ONLY, repo.getByCloudId("other")!!.syncStatus)
    }

    @Test
    fun `rescanning a synced copy keeps server metadata and cloud visibility`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.upsert(
                MediaEntity(
                    id = "local_1",
                    cloudId = "c1",
                    localContentUri = "content://media/external/media/1",
                    bucketId = "old",
                    sizeBytes = 9_999L,
                    ratio = 1.777f,
                    durationMs = 12_345,
                    sortTimestamp = 1_600_000_000_000L,
                    yearMonth = 202009,
                    isVisibleInTimeline = true,
                    syncStatus = SyncStatus.SYNCED
                )
            )
            val store = FakeLocalMediaStore().apply {
                setImageFiles(
                    listOf(
                        file(
                            id = 1,
                            relativePath = "DCIM/Screenshots/",
                            bucketId = "shots",
                            width = 0,
                            height = 0
                        )
                    )
                )
            }

            useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

            val row = repo.getById("local_1")!!
            assertEquals("shots", row.bucketId)
            assertEquals(9_999L, row.sizeBytes)
            assertEquals(1.777f, row.ratio, 0.0001f)
            assertEquals(12_345, row.durationMs)
            assertEquals(1_600_000_000_000L, row.sortTimestamp)
            assertEquals(202009, row.yearMonth)
            assertTrue(row.isVisibleInTimeline)
            assertEquals(SyncStatus.SYNCED, row.syncStatus)
        }

    @Test
    fun `moving a synced file to a new MediaStore URI re-links it to the cloud row`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.upsert(
                MediaEntity(
                    id = "local_1",
                    cloudId = "c1",
                    localContentUri = "content://media/external/media/1",
                    bucketId = "camera",
                    sizeBytes = 1_002L,
                    ratio = 4000f / 3000f,
                    sortTimestamp = 1_730_000_000_000L,
                    yearMonth = 202410,
                    isVisibleInTimeline = true,
                    syncStatus = SyncStatus.SYNCED
                )
            )
            // Old URI 1 is gone; same file now lives at URI 2 with matching size/ratio/timestamp.
            val movedFile = file(2).copy(sizeBytes = 1_002L)
            val store = FakeLocalMediaStore().apply { setImageFiles(listOf(movedFile)) }

            val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

            assertEquals(1, result.removed)
            assertEquals(1, result.deduplicated)
            assertNull(repo.getById("local_2"))

            val synced = repo.getByCloudId("c1")!!
            assertEquals("content://media/external/media/2", synced.localContentUri)
            assertEquals(SyncStatus.SYNCED, synced.syncStatus)
            assertTrue(synced.isVisibleInTimeline)
        }

    @Test
    fun `removing a visible sibling while rescanning keeps the promoted copy visible`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.upsert(
                MediaEntity(
                    id = "local_gone",
                    cloudId = "c1",
                    localContentUri = "content://media/external/media/99",
                    bucketId = "camera",
                    isVisibleInTimeline = true,
                    syncStatus = SyncStatus.SYNCED
                )
            )
            repo.upsert(
                MediaEntity(
                    id = "local_keep",
                    cloudId = "c1",
                    localContentUri = "content://media/external/media/1",
                    bucketId = "old_bucket",
                    isVisibleInTimeline = false,
                    syncStatus = SyncStatus.SYNCED
                )
            )
            val store = FakeLocalMediaStore().apply {
                setImageFiles(listOf(file(1, relativePath = "DCIM/NewFolder/", bucketId = "new_bucket")))
            }

            useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

            assertNull(repo.getById("local_gone"))
            val kept = repo.getById("local_keep")!!
            assertTrue(kept.isVisibleInTimeline)
            assertEquals("new_bucket", kept.bucketId)
        }

    @Test
    fun `rescanning a local row keeps auto backup suppression`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "local_1",
                localContentUri = "content://media/external/media/1",
                autoBackupSuppressed = true,
                sortTimestamp = 1_600_000_000_000L,
                syncStatus = SyncStatus.LOCAL_ONLY
            )
        )
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        val row = repo.getById("local_1")!!
        assertTrue(row.autoBackupSuppressed)
        assertEquals(1_730_000_000_000L, row.sortTimestamp)
    }

    @Test
    fun `zone provider is read on each scan instead of being cached at construction`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            val folders = FakeDeviceFolderRepository()
            // 2024-10-31T23:23Z is October in UTC, November in Europe/Amsterdam (UTC+1).
            val timestamp = 1_730_417_000_000L
            var currentZone = ZoneId.of("UTC")
            val store = FakeLocalMediaStore().apply {
                setImageFiles(listOf(file(1, dateTakenMs = timestamp)))
            }
            val scan = useCase(store, repo, folders) { currentZone }

            scan(fullAccess()).getOrThrow()
            assertEquals(202410, repo.getById("local_1")!!.yearMonth)

            currentZone = ZoneId.of("Europe/Amsterdam")
            store.setImageFiles(
                listOf(file(1, dateTakenMs = timestamp), file(2, dateTakenMs = timestamp)),
                itemGeneration = 2L
            )
            store.generation = 2L
            scan(fullAccess()).getOrThrow()

            assertEquals(202411, repo.getById("local_2")!!.yearMonth)
        }

    @Test
    fun `partial access never deletes rows missing from the cursor`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "local_old",
                localContentUri = "content://media/external/media/99",
                syncStatus = SyncStatus.LOCAL_ONLY
            )
        )
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(
            MediaAccessState(images = MediaAccess.PARTIAL, videos = MediaAccess.PARTIAL)
        ).getOrThrow()

        assertEquals(0, result.removed)
        assertNotNull(repo.getById("local_old"))
    }

    @Test
    fun `an empty cursor with local rows is not trusted`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "local_old",
                localContentUri = "content://media/external/media/99",
                syncStatus = SyncStatus.LOCAL_ONLY
            )
        )

        val result = useCase(FakeLocalMediaStore(), repo, FakeDeviceFolderRepository())(fullAccess())
            .getOrThrow()

        assertEquals(0, result.removed)
        assertNotNull(repo.getById("local_old"))
    }

    @Test
    fun `full access drops a local-only row whose file is gone`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "local_gone",
                localContentUri = "content://media/external/media/99",
                syncStatus = SyncStatus.LOCAL_ONLY
            )
        )
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        assertEquals(1, result.removed)
        assertNull(repo.getById("local_gone"))
    }

    @Test
    fun `full access keeps the cloud row when the last local copy disappears`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.upsert(
                MediaEntity(
                    id = "local_gone",
                    cloudId = "c1",
                    localContentUri = "content://media/external/media/99",
                    syncStatus = SyncStatus.SYNCED
                )
            )
            val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

            useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

            val row = repo.getByCloudId("c1")!!
            assertEquals(SyncStatus.CLOUD_ONLY, row.syncStatus)
            assertNull(row.localContentUri)
            assertTrue(row.isVisibleInTimeline)
        }

    @Test
    fun `full access promotes a sibling copy when the visible one is gone`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "local_gone",
                cloudId = "c1",
                localContentUri = "content://media/external/media/99",
                syncStatus = SyncStatus.SYNCED,
                isVisibleInTimeline = true
            )
        )
        repo.upsert(
            MediaEntity(
                id = "local_keep",
                cloudId = "c1",
                localContentUri = "content://media/external/media/1",
                syncStatus = SyncStatus.SYNCED,
                isVisibleInTimeline = false
            )
        )
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        useCase(store, repo, FakeDeviceFolderRepository())(fullAccess()).getOrThrow()

        assertNull(repo.getById("local_gone"))
        assertTrue(repo.getById("local_keep")!!.isVisibleInTimeline)
        assertEquals(SyncStatus.SYNCED, repo.getById("local_keep")!!.syncStatus)
    }

    @Test
    fun `no access does nothing at all`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }

        val result = useCase(store, repo, FakeDeviceFolderRepository())(MediaAccessState()).getOrThrow()

        assertEquals(0, result.scanned)
        assertTrue(repo.items.isEmpty())
    }

    @Test
    fun `unchanged media store generation skips the scan`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1)), itemGeneration = 1L) }
        val scanState = FakeScanStateStore()
        val scan = useCase(store, repo, FakeDeviceFolderRepository(), scanState)

        assertEquals(1, scan(fullAccess()).getOrThrow().scanned)
        assertEquals("${AppDatabase.VERSION}|v1|FULL|FULL|UTC|1", scanState.signature)

        // Same generation: the added file is not picked up because MediaStore did not change.
        store.setImageFiles(listOf(file(1), file(2)), itemGeneration = 1L)
        assertEquals(0, scan(fullAccess()).getOrThrow().scanned)
        assertNull(repo.getById("local_2"))

        // Generation bumps: incremental scan queries only items modified after generation 1.
        store.generation = 2L
        store.images = listOf(1L to file(1), 2L to file(2))
        val incrementalResult = scan(fullAccess()).getOrThrow()
        assertEquals(1, incrementalResult.scanned)
        assertEquals(listOf(null, 1L), store.lastModifiedAfterRequests)
        assertNotNull(repo.getById("local_2"))
    }

    @Test
    fun `incremental scan detects deleted local files via live uri check`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeLocalMediaStore().apply {
            images = listOf(1L to file(1), 1L to file(2))
        }
        val scanState = FakeScanStateStore()
        val scan = useCase(store, repo, FakeDeviceFolderRepository(), scanState)

        assertEquals(2, scan(fullAccess()).getOrThrow().scanned)

        // File 2 is deleted and generation bumps to 2 with no new additions.
        store.images = listOf(1L to file(1))
        store.generation = 2L
        val result = scan(fullAccess()).getOrThrow()

        assertEquals(0, result.scanned)
        assertEquals(1, result.removed)
        assertNotNull(repo.getById("local_1"))
        assertNull(repo.getById("local_2"))
    }

    @Test
    fun `media store version change forces a full scan`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeLocalMediaStore().apply {
            images = listOf(1L to file(1))
        }
        val scanState = FakeScanStateStore()
        val scan = useCase(store, repo, FakeDeviceFolderRepository(), scanState)

        scan(fullAccess()).getOrThrow()

        store.version = "v2"
        store.generation = 2L
        store.images = listOf(1L to file(1), 2L to file(2))
        val result = scan(fullAccess()).getOrThrow()

        assertEquals(2, result.scanned)
        assertEquals(listOf(null, null), store.lastModifiedAfterRequests)
    }

    @Test
    fun `partial access always scans and never records a signature`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeLocalMediaStore().apply { setImageFiles(listOf(file(1))) }
        val scanState = FakeScanStateStore()
        val scan = useCase(store, repo, FakeDeviceFolderRepository(), scanState)
        val partial = MediaAccessState(MediaAccess.PARTIAL, MediaAccess.PARTIAL)

        assertEquals(1, scan(partial).getOrThrow().scanned)
        assertNull(scanState.signature)
        assertEquals(1, scan(partial).getOrThrow().scanned)
    }

    private fun fullAccess() = MediaAccessState(MediaAccess.FULL, MediaAccess.FULL)
}