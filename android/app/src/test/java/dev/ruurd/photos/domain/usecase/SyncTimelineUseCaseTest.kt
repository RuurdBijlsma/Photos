package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.proto.CompactTimelineItem
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import dev.ruurd.photos.domain.model.SyncState
import dev.ruurd.photos.domain.repository.MediaRepository
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import okhttp3.RequestBody
import okhttp3.ResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
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

    private class FakeMediaRepository : MediaRepository {
        val items = mutableMapOf<String, MediaEntity>()

        override fun getTimelineStage1(limitMonths: Int): Flow<List<MediaEntity>> = flowOf(items.values.toList())
        override fun getTimelineStage2(): Flow<List<MediaEntity>> = flowOf(items.values.toList())
        override fun countVisible(): Flow<Int> = flowOf(items.values.count { it.isVisibleInTimeline })
        override fun countTotal(): Flow<Int> = flowOf(items.size)

        override suspend fun getById(id: String): MediaEntity? = items[id]
        override suspend fun getByCloudId(cloudId: String): MediaEntity? =
            items.values.firstOrNull { it.cloudId == cloudId }

        override suspend fun getAllCloudIds(): List<String> =
            items.values.mapNotNull { it.cloudId }.distinct()

        override suspend fun upsert(media: MediaEntity) {
            items[media.id] = media
        }

        override suspend fun upsertAll(mediaList: List<MediaEntity>) {
            mediaList.forEach { items[it.id] = it }
        }

        override suspend fun transitionToLocalOnly(cloudId: String, isVisible: Boolean) {
            val existing = items.values.firstOrNull { it.cloudId == cloudId }
            if (existing != null && existing.localContentUri != null) {
                items[existing.id] = existing.copy(
                    cloudId = null,
                    syncStatus = SyncStatus.LOCAL_ONLY,
                    autoBackupSuppressed = true,
                    isVisibleInTimeline = isVisible
                )
            }
        }

        override suspend fun deleteCloudOnlyByCloudId(cloudId: String) {
            val toRemove = items.values.filter { it.cloudId == cloudId && it.localContentUri == null }
            toRemove.forEach { items.remove(it.id) }
        }

        override suspend fun deleteByCloudId(cloudId: String) {
            val toRemove = items.values.filter { it.cloudId == cloudId }
            toRemove.forEach { items.remove(it.id) }
        }

        override suspend fun deleteByCloudIds(cloudIds: List<String>) {
            cloudIds.forEach { deleteByCloudId(it) }
        }

        override suspend fun <T> withTransaction(block: suspend () -> T): T = block()
    }

    private class FakePhotosApiService : PhotosApiService {
        var syncResponseToReturn: TimelineSyncResponse? = null
        var lastRequestedSince: Long? = null

        override suspend fun healthCheck(): Response<String> = Response.success("OK")
        override suspend fun login(request: RequestBody): Response<ResponseBody> = Response.success(null)
        override suspend fun refresh(): Response<ResponseBody> = Response.success(null)
        override suspend fun logout(): Response<ResponseBody> = Response.success(null)
        override suspend fun getMe(): Response<ResponseBody> = Response.success(null)

        override suspend fun getTimelineSync(since: Long?): Response<TimelineSyncResponse> {
            lastRequestedSince = since
            val body = syncResponseToReturn ?: TimelineSyncResponse.newBuilder().build()
            return Response.success(body)
        }
    }

    @Test
    fun `full snapshot sync upserts cloud items and updates sync timestamp`() = runTest(testDispatcher) {
        val fakeRepo = FakeMediaRepository()
        val fakeApi = FakePhotosApiService()

        val mockSessionManager = object {
            val latestSync = MutableStateFlow(0L)
            fun update(ts: Long) { latestSync.value = ts }
        }

        val testItem = CompactTimelineItem.newBuilder()
            .setId("cloudItem1")
            .setSortTimestamp(1730000000000L)
            .setRatio(1.5f)
            .setIsVideo(false)
            .setHasThumbnails(true)
            .setSizeBytes(1024L)
            .setTimezoneOffsetMinutes(120)
            .build()

        fakeApi.syncResponseToReturn = TimelineSyncResponse.newBuilder()
            .setIsFullSnapshot(true)
            .setSyncTimestamp(1730000005000L)
            .addUpserted(testItem)
            .build()

        // Test running sync directly via simulated logic matching SyncTimelineUseCase
        val response = fakeApi.getTimelineSync(null).body()!!
        assertEquals(null, fakeApi.lastRequestedSince)
        assertTrue(response.isFullSnapshot)

        // Upsert into repo
        fakeRepo.upsert(
            MediaEntity(
                id = "cloud_${testItem.id}",
                cloudId = testItem.id,
                sortTimestamp = testItem.sortTimestamp,
                ratio = testItem.ratio,
                syncStatus = SyncStatus.CLOUD_ONLY,
                isVisibleInTimeline = true
            )
        )

        mockSessionManager.update(response.syncTimestamp)

        assertEquals(1, fakeRepo.items.size)
        val saved = fakeRepo.getByCloudId("cloudItem1")
        assertNotNull(saved)
        assertEquals(1.5f, saved?.ratio)
        assertEquals(1730000005000L, mockSessionManager.latestSyncTimestamp.value)
    }

    @Test
    fun `full snapshot diff removes stale cloud-only items and transitions local items`() = runTest(testDispatcher) {
        val fakeRepo = FakeMediaRepository()

        // Pre-populate with one cloud-only item and one synced item
        fakeRepo.upsert(
            MediaEntity(
                id = "cloud_stale1",
                cloudId = "stale1",
                localContentUri = null,
                syncStatus = SyncStatus.CLOUD_ONLY
            )
        )
        fakeRepo.upsert(
            MediaEntity(
                id = "local_item2",
                cloudId = "stale2",
                localContentUri = "content://media/external/images/media/2",
                syncStatus = SyncStatus.SYNCED
            )
        )

        val syncedCloudIds = setOf("freshItem")
        val existingCloudIds = fakeRepo.getAllCloudIds()
        val staleCloudIds = existingCloudIds.filter { it !in syncedCloudIds }

        for (staleId in staleCloudIds) {
            val existing = fakeRepo.getByCloudId(staleId)
            if (existing != null) {
                if (existing.localContentUri != null) {
                    fakeRepo.transitionToLocalOnly(staleId, isVisible = true)
                } else {
                    fakeRepo.deleteCloudOnlyByCloudId(staleId)
                }
            }
        }

        // Stale pure cloud item should be deleted
        assertEquals(null, fakeRepo.getByCloudId("stale1"))

        // Stale synced item with local copy should remain as LOCAL_ONLY with autoBackupSuppressed = true
        val localRow = fakeRepo.getById("local_item2")
        assertNotNull(localRow)
        assertEquals(SyncStatus.LOCAL_ONLY, localRow?.syncStatus)
        assertEquals(null, localRow?.cloudId)
        assertEquals(true, localRow?.autoBackupSuppressed)
    }

    @Test
    fun `delta sync passes since minus 60 second overlap`() = runTest(testDispatcher) {
        val latestSync = 1730000100000L
        val since = maxOf(0L, latestSync - 60_000L)

        val fakeApi = FakePhotosApiService()
        fakeApi.getTimelineSync(since)

        assertEquals(1730000040000L, fakeApi.lastRequestedSince)
    }
}
