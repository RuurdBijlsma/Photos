package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.network.SyncTimestampStore
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.proto.CompactTimelineItem
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.testing.FakeMediaRepository
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import okhttp3.RequestBody
import okhttp3.ResponseBody
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Response

@OptIn(ExperimentalCoroutinesApi::class)
class SyncTimelineUseCaseTest {

    private val testDispatcher = StandardTestDispatcher()

    private val dispatchers = object : AppDispatchers {
        override val default: CoroutineDispatcher = testDispatcher
        override val io: CoroutineDispatcher = testDispatcher
        override val main: CoroutineDispatcher = testDispatcher
    }

    private class FakeSyncTimestampStore : SyncTimestampStore {
        override val latestSyncTimestamp = MutableStateFlow(0L)

        override suspend fun setLatestSyncTimestamp(timestamp: Long) {
            latestSyncTimestamp.value = timestamp
        }
    }

    private class FakePhotosApiService : PhotosApiService {
        var response: Response<TimelineSyncResponse> =
            Response.success(TimelineSyncResponse.getDefaultInstance())
        var lastRequestedSince: Long? = null

        override suspend fun healthCheck(): Response<String> = Response.success("OK")

        override suspend fun login(request: RequestBody): Response<ResponseBody> =
            Response.success(null)

        override suspend fun refresh(): Response<ResponseBody> = Response.success(null)

        override suspend fun logout(): Response<ResponseBody> = Response.success(null)

        override suspend fun getMe(): Response<ResponseBody> = Response.success(null)

        override suspend fun getTimelineSync(since: Long?): Response<TimelineSyncResponse> {
            lastRequestedSince = since
            return response
        }
    }

    private fun item(id: String, sortTimestamp: Long = 1_730_000_000_000L) =
        CompactTimelineItem.newBuilder()
            .setId(id)
            .setSortTimestamp(sortTimestamp)
            .setRatio(1.5f)
            .setIsVideo(false)
            .setHasThumbnails(true)
            .setSizeBytes(1024L)
            .setTimezoneOffsetMinutes(120)
            .build()

    private fun useCase(
        api: FakePhotosApiService,
        repo: FakeMediaRepository,
        store: FakeSyncTimestampStore
    ) = SyncTimelineUseCase(api, repo, store, dispatchers)

    @Test
    fun `full snapshot upserts cloud items and persists the sync timestamp`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            val store = FakeSyncTimestampStore()
            val api = FakePhotosApiService().apply {
                response = Response.success(
                    TimelineSyncResponse.newBuilder()
                        .setIsFullSnapshot(true)
                        .setSyncTimestamp(1_730_000_005_000L)
                        .addUpserted(item("a1B2c3D4e5"))
                        .build()
                )
            }

            val useCase = useCase(api, repo, store)
            val result = useCase()

            assertTrue(result.isSuccess)
            assertNull(api.lastRequestedSince)

            val saved = repo.getByCloudId("a1B2c3D4e5")!!
            assertEquals(SyncStatus.CLOUD_ONLY, saved.syncStatus)
            assertNull(saved.localContentUri)
            assertEquals(1.5f, saved.ratio)
            assertEquals(202410, saved.yearMonth)
            assertEquals(true, saved.isVisibleInTimeline)

            assertEquals(1_730_000_005_000L, store.latestSyncTimestamp.value)
            assertEquals(SyncState.IDLE, useCase.syncState.value)
        }

    @Test
    fun `delta sync uses a 60 second overlap and applies upserts and deletions`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            val store = FakeSyncTimestampStore().apply {
                latestSyncTimestamp.value = 1_730_000_100_000L
            }
            val api = FakePhotosApiService().apply {
                response = Response.success(
                    TimelineSyncResponse.newBuilder()
                        .setIsFullSnapshot(false)
                        .setSyncTimestamp(1_730_000_200_000L)
                        .addDeletedIds("staleCloudOnly")
                        .addDeletedIds("staleSynced")
                        .addUpserted(item("keepMe"))
                        .build()
                )
            }

            repo.upsert(MediaEntity(id = "cloud_staleCloudOnly", cloudId = "staleCloudOnly"))
            repo.upsert(
                MediaEntity(
                    id = "local_staleSynced",
                    cloudId = "staleSynced",
                    localContentUri = "content://media/external/images/media/2",
                    syncStatus = SyncStatus.SYNCED
                )
            )
            repo.upsert(
                MediaEntity(
                    id = "local_keepMe",
                    cloudId = "keepMe",
                    localContentUri = "content://media/external/images/media/3",
                    ratio = 1f,
                    syncStatus = SyncStatus.SYNCED
                )
            )
            repo.upsert(
                MediaEntity(
                    id = "local_keepMeDup",
                    cloudId = "keepMe",
                    localContentUri = "content://media/external/images/media/9",
                    ratio = 1f,
                    isVisibleInTimeline = false,
                    syncStatus = SyncStatus.SYNCED
                )
            )

            val result = useCase(api, repo, store)()

            assertTrue(result.isSuccess)
            assertEquals(1_730_000_040_000L, api.lastRequestedSince)

            assertNull(repo.getByCloudId("staleCloudOnly"))

            val reverted = repo.getById("local_staleSynced")!!
            assertEquals(SyncStatus.LOCAL_ONLY, reverted.syncStatus)
            assertNull(reverted.cloudId)
            assertEquals(true, reverted.autoBackupSuppressed)
            assertEquals(true, reverted.isVisibleInTimeline)

            val updated = repo.getById("local_keepMe")!!
            assertEquals(1.5f, updated.ratio)
            assertEquals("content://media/external/images/media/3", updated.localContentUri)
            assertEquals(SyncStatus.SYNCED, updated.syncStatus)

            val updatedDup = repo.getById("local_keepMeDup")!!
            assertEquals(1.5f, updatedDup.ratio)
            assertEquals(false, updatedDup.isVisibleInTimeline)

            assertEquals(1_730_000_200_000L, store.latestSyncTimestamp.value)
        }

    @Test
    fun `full snapshot reconciles rows missing from the snapshot`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeSyncTimestampStore()
        val api = FakePhotosApiService().apply {
            response = Response.success(
                TimelineSyncResponse.newBuilder()
                    .setIsFullSnapshot(true)
                    .setSyncTimestamp(1_730_000_300_000L)
                    .addUpserted(item("stillOnServer"))
                    .build()
            )
        }

        repo.upsert(MediaEntity(id = "cloud_gone", cloudId = "gone"))
        repo.upsert(
            MediaEntity(
                id = "local_gone",
                cloudId = "goneLocal",
                localContentUri = "content://media/external/images/media/4",
                syncStatus = SyncStatus.SYNCED
            )
        )

        val result = useCase(api, repo, store)()

        assertTrue(result.isSuccess)
        assertNull(repo.getByCloudId("gone"))
        assertEquals(SyncStatus.LOCAL_ONLY, repo.getById("local_gone")!!.syncStatus)
        assertEquals(true, repo.getById("local_gone")!!.autoBackupSuppressed)
        assertNotNull(repo.getByCloudId("stillOnServer"))
    }

    @Test
    fun `local row visibility follows its device folder after a cloud deletion`() =
        runTest(testDispatcher) {
            val repo = FakeMediaRepository()
            repo.folderVisibility["shots"] = false
            repo.upsert(
                MediaEntity(
                    id = "local_shot",
                    cloudId = "c9",
                    localContentUri = "content://media/external/images/media/9",
                    bucketId = "shots",
                    syncStatus = SyncStatus.SYNCED
                )
            )
            val api = FakePhotosApiService().apply {
                response = Response.success(
                    TimelineSyncResponse.newBuilder()
                        .setSyncTimestamp(1_730_000_400_000L)
                        .addDeletedIds("c9")
                        .build()
                )
            }

            val result = useCase(api, repo, FakeSyncTimestampStore())()

            assertTrue(result.isSuccess)
            val reverted = repo.getById("local_shot")!!
            assertEquals(SyncStatus.LOCAL_ONLY, reverted.syncStatus)
            assertEquals(false, reverted.isVisibleInTimeline)
        }

    @Test
    fun `server error fails the sync and keeps the stored timestamp`() = runTest(testDispatcher) {
        val repo = FakeMediaRepository()
        val store = FakeSyncTimestampStore()
        val api = FakePhotosApiService().apply {
            response = Response.error(503, "".toResponseBody())
        }

        val useCase = useCase(api, repo, store)
        val result = useCase()

        assertTrue(result.isFailure)
        assertEquals(SyncState.SERVER_UNREACHABLE, useCase.syncState.value)
        assertEquals(0L, store.latestSyncTimestamp.value)
        assertTrue(repo.items.isEmpty())
    }
}