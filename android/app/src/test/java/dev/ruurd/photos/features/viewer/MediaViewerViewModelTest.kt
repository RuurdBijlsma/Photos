package dev.ruurd.photos.features.viewer

import android.content.Intent
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.MediaFileSharer
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.core.network.dto.UpdateMediaItemRequestDto
import dev.ruurd.photos.data.datasource.local.LocalFileOverwriteHelper
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoSource
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.VideoQuality
import dev.ruurd.photos.domain.model.VideoUnavailableReason
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import dev.ruurd.photos.domain.repository.MediaMutationRepository
import dev.ruurd.photos.domain.usecase.RotateMediaUseCase
import dev.ruurd.photos.testing.FakeMediaRepository
import dev.ruurd.photos.testing.TestAppDispatchers
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class MediaViewerViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private val testScope = TestScope(testDispatcher)

    private val dispatchers = TestAppDispatchers(testDispatcher)

    private val detailsRepository = FakeMediaDetailsRepository(
        mapOf(
            "c1" to MediaDetails(cloudId = "c1", fileName = "one.jpg", mimeType = "image/jpeg"),
            "c2" to MediaDetails(cloudId = "c2", fileName = "two.jpg", mimeType = "image/jpeg"),
            "c3" to MediaDetails(cloudId = "c3", fileName = "three.jpg", mimeType = "image/jpeg"),
        ),
    )

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `initializes with warm store and anchors initial page`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p2", state.activeMediaId)
        assertEquals(3, state.items.size)
        assertEquals(1, state.initialIndex)

        // Page change updates activeMediaId and initialIndex
        viewModel.onPageChanged("p3")
        assertEquals("p3", viewModel.uiState.value.activeMediaId)
        assertEquals(2, viewModel.uiState.value.initialIndex)

        // Chrome toggle
        assertTrue(viewModel.uiState.value.isChromeVisible)
        viewModel.toggleChrome()
        assertFalse(viewModel.uiState.value.isChromeVisible)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `re-anchors initialIndex when store transitions from Loading fallback to Loaded`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        // Store has not collected yet -> starts in Loading
        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        val state = viewModel.uiState.value
        assertEquals("p2", state.activeMediaId)
        assertEquals(3, state.items.size)
        assertEquals(1, state.initialIndex)
        assertFalse(state.isActiveItemMissing)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `keeps activeMediaId anchored when newer items are prepended`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()
        assertEquals(1, viewModel.uiState.value.initialIndex)

        // Prepend a newer item -> p2 shifts from index 1 to index 2
        repo.upsert(MediaEntity(id = "p0", sortTimestamp = 4000L, yearMonth = 202611))
        runCurrent()

        val updated = viewModel.uiState.value
        assertEquals("p2", updated.activeMediaId)
        assertEquals(4, updated.items.size)
        assertEquals(2, updated.initialIndex)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `clamps to neighbor when active item is deleted and flags missing when library empties`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", sortTimestamp = 2000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        repo.deleteById("p2")
        runCurrent()

        val clamped = viewModel.uiState.value
        assertEquals("p1", clamped.activeMediaId)
        assertEquals(0, clamped.initialIndex)
        assertFalse(clamped.isActiveItemMissing)

        repo.deleteById("p1")
        runCurrent()

        assertTrue(viewModel.uiState.value.isActiveItemMissing)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `loads active details immediately and prefetches neighbors on settle`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", cloudId = "c1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", cloudId = "c2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", cloudId = "c3", sortTimestamp = 1000L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val localDetailsRepo = FakeMediaDetailsRepository(
            mapOf(
                "c1" to MediaDetails(cloudId = "c1", fileName = "one.jpg", mimeType = "image/jpeg"),
                "c2" to MediaDetails(cloudId = "c2", fileName = "two.jpg", mimeType = "image/jpeg"),
                "c3" to MediaDetails(cloudId = "c3", fileName = "three.jpg", mimeType = "image/jpeg"),
            ),
        )
        val fullResStore = FakeFullResFileStore()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = localDetailsRepo,
            fullResFileCache = fullResStore,
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        // Active page details load immediately without waiting for onSettledPage.
        assertEquals("c2", viewModel.uiState.value.activeDetails?.cloudId)
        assertEquals(listOf("c2"), localDetailsRepo.requested)

        viewModel.onSettledPage("p2")
        runCurrent()

        assertEquals("c1", viewModel.uiState.value.detailsById["p1"]?.cloudId)
        assertEquals("c3", viewModel.uiState.value.detailsById["p3"]?.cloudId)
        assertTrue(localDetailsRepo.requested.containsAll(listOf("c2", "c1", "c3")))
        assertEquals(listOf("c2"), fullResStore.requestedCloudIds)

        // Swiping to a prefetched neighbor keeps its details available immediately.
        viewModel.onPageChanged("p3")
        assertEquals("c3", viewModel.uiState.value.activeDetails?.cloudId)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `swiping away cancels pending settle neighbor prefetch`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(id = "p1", cloudId = "c1", sortTimestamp = 3000L, yearMonth = 202611),
                MediaEntity(id = "p2", cloudId = "c2", sortTimestamp = 2000L, yearMonth = 202611),
                MediaEntity(id = "p3", cloudId = "c3", sortTimestamp = 1000L, yearMonth = 202611),
                MediaEntity(id = "p4", cloudId = "c4", sortTimestamp = 500L, yearMonth = 202611),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val slowDetailsRepo = FakeMediaDetailsRepository(
            items = mapOf(
                "c1" to MediaDetails(cloudId = "c1"),
                "c2" to MediaDetails(cloudId = "c2"),
                "c3" to MediaDetails(cloudId = "c3"),
                "c4" to MediaDetails(cloudId = "c4"),
            ),
            delayMs = 500L,
        )
        val viewModel = MediaViewerViewModel(
            initialMediaId = "p2",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = slowDetailsRepo,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        advanceTimeBy(500L)
        runCurrent()

        viewModel.onSettledPage("p2")
        advanceTimeBy(100L)
        runCurrent()

        // Swipe to p3 before p2's neighbor prefetch finishes: c1 should never be published.
        viewModel.onPageChanged("p3")
        advanceTimeBy(500L)
        runCurrent()

        assertNull(viewModel.uiState.value.detailsById["p1"])
        assertEquals("c3", viewModel.uiState.value.activeDetails?.cloudId)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `synced item skips local exif read when server details succeed and falls back when offline`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(
                    id = "local_1",
                    cloudId = "c1",
                    localContentUri = "content://media/external/images/media/1",
                    syncStatus = SyncStatus.SYNCED,
                    sortTimestamp = 3000L,
                    yearMonth = 202611,
                ),
                MediaEntity(
                    id = "local_2",
                    cloudId = "c_missing",
                    localContentUri = "content://media/external/images/media/2",
                    syncStatus = SyncStatus.SYNCED,
                    sortTimestamp = 2000L,
                    yearMonth = 202611,
                ),
            ),
        )

        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val localSource = FakeLocalMediaInfoSource(
            LocalMediaInfo(fileName = "local_fallback.jpg", sizeBytes = 4096L),
        )
        val fullResStore = FakeFullResFileStore()
        val viewModel = MediaViewerViewModel(
            initialMediaId = "local_1",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = fullResStore,
            localMediaInfoReader = localSource,
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
        )
        runCurrent()

        assertTrue(localSource.requestedUris.isEmpty())

        viewModel.onSettledPage("local_1")
        runCurrent()
        assertTrue(fullResStore.requestedCloudIds.isEmpty())

        // local_2 has no server details (offline/missing) -> falls back to local EXIF/MediaStore.
        assertEquals(listOf("content://media/external/images/media/2"), localSource.requestedUris)
        assertEquals("local_fallback.jpg", viewModel.uiState.value.localInfoById["local_2"]?.fileName)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `opening directly on video hides chrome initially and supports video preferences`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsert(
            MediaEntity(
                id = "v1",
                cloudId = "c1",
                isVideo = true,
                durationMs = 15_000,
                hasThumbnails = true,
                sortTimestamp = 3000L,
                yearMonth = 202611,
            ),
        )
        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "v1",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
            videoPreferencesStore = InMemoryVideoPreferencesSource(),
        )
        runCurrent()

        assertFalse(viewModel.uiState.value.isChromeVisible)

        viewModel.toggleVideoMute()
        viewModel.toggleVideoLoop()
        viewModel.setVideoPlaybackSpeed(1.5f)
        runCurrent()

        val prefs = viewModel.uiState.value.videoPreferences
        assertTrue(prefs.isMuted)
        assertTrue(prefs.isLoopEnabled)
        assertEquals(1.5f, prefs.playbackSpeed, 0.001f)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `handles local fallback, 480p fallback, quality selection, and error retry`() = testScope.runTest {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(
                    id = "v_synced",
                    cloudId = "c1",
                    localContentUri = "content://media/external/video/1",
                    isVideo = true,
                    hasThumbnails = true,
                    syncStatus = SyncStatus.SYNCED,
                    sortTimestamp = 3000L,
                    yearMonth = 202611,
                ),
                MediaEntity(
                    id = "v_local",
                    cloudId = null,
                    localContentUri = "content://media/external/video/2",
                    isVideo = true,
                    syncStatus = SyncStatus.LOCAL_ONLY,
                    sortTimestamp = 2000L,
                    yearMonth = 202611,
                ),
            ),
        )
        val store = TimelineProjectionStore(
            mediaRepository = repo,
            scope = backgroundScope,
            dispatchers = dispatchers,
        )
        runCurrent()

        val viewModel = MediaViewerViewModel(
            initialMediaId = "v_synced",
            timelineProjectionStore = store,
            mediaRepository = repo,
            mediaDetailsRepository = detailsRepository,
            fullResFileCache = FakeFullResFileStore(),
            localMediaInfoReader = FakeLocalMediaInfoSource(),
            mediaFileSharer = FakeMediaFileSharer(),
            dispatchers = dispatchers,
            videoPreferencesStore = InMemoryVideoPreferencesSource(),
        )
        runCurrent()

        // Synced video falls back to cloud when local URI fails; local-only sets an error.
        viewModel.onLocalVideoFailed("v_synced")
        viewModel.onLocalVideoFailed("v_local")
        assertTrue("v_synced" in viewModel.uiState.value.localVideoFallbackIds)
        assertNull(viewModel.uiState.value.videoErrorById["v_synced"])
        assertEquals(
            VideoUnavailableReason.PLAYBACK_FAILED,
            viewModel.uiState.value.videoErrorById["v_local"],
        )

        // Cloud original failure triggers transient 480p override without changing saved pref.
        viewModel.onCloudVideoFallbackTo480p("v_synced")
        assertEquals(VideoQuality.P480, viewModel.uiState.value.transientVideoQualityById["v_synced"])
        assertTrue("v_synced" in viewModel.uiState.value.videoUnavailableOriginalIds)
        assertEquals(VideoQuality.AUTO, viewModel.uiState.value.videoPreferences.preferredQuality)

        // Cloud error and retry clear videoErrorById.
        viewModel.onCloudVideoError("v_synced", VideoUnavailableReason.OFFLINE)
        assertEquals(VideoUnavailableReason.OFFLINE, viewModel.uiState.value.videoErrorById["v_synced"])
        viewModel.retryActiveItemIfNeeded()
        assertNull(viewModel.uiState.value.videoErrorById["v_synced"])

        // Selecting a quality persists preference and clears transient override.
        viewModel.selectVideoQuality("v_synced", VideoQuality.P144)
        runCurrent()
        assertNull(viewModel.uiState.value.transientVideoQualityById["v_synced"])
        assertEquals(VideoQuality.P144, viewModel.uiState.value.videoPreferences.preferredQuality)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `rotate tap turns optimistically and commits once after the debounce`() = testScope.runTest {
        val repo = rotationRepository()
        val store = TimelineProjectionStore(repo, backgroundScope, dispatchers)
        runCurrent()
        val mutations = FakeRotationMutationRepository("c1b")
        val viewModel = createRotationViewModel(repo, store, mutations)
        runCurrent()

        viewModel.rotateActiveItem()
        assertEquals(90, viewModel.uiState.value.pendingRotationById["cloud_c1"])
        assertEquals(0, mutations.updateCalls)

        advanceTimeBy(2_501L)
        runCurrent()

        assertEquals(1, mutations.updateCalls)
        assertEquals("cloud_c1b", viewModel.uiState.value.activeMediaId)
        assertTrue(viewModel.uiState.value.pendingRotationById.isEmpty())
        assertTrue(viewModel.uiState.value.mutatingIds.isEmpty())
        assertEquals(6, viewModel.uiState.value.detailsById["cloud_c1b"]?.orientation)
        assertEquals("c1b", repo.getById("cloud_c1b")?.cloudId)
        assertNull(repo.getById("cloud_c1"))

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `four rotate taps cancel out without a request`() = testScope.runTest {
        val repo = rotationRepository()
        val store = TimelineProjectionStore(repo, backgroundScope, dispatchers)
        runCurrent()
        val mutations = FakeRotationMutationRepository("c1b")
        val viewModel = createRotationViewModel(repo, store, mutations)
        runCurrent()

        repeat(4) { viewModel.rotateActiveItem() }
        assertEquals(0, viewModel.uiState.value.pendingRotationById["cloud_c1"])

        advanceTimeBy(2_501L)
        runCurrent()

        assertEquals(0, mutations.updateCalls)
        assertTrue(viewModel.uiState.value.pendingRotationById.isEmpty())

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `flush commits a pending rotation immediately`() = testScope.runTest {
        val repo = rotationRepository()
        val store = TimelineProjectionStore(repo, backgroundScope, dispatchers)
        runCurrent()
        val mutations = FakeRotationMutationRepository("c1b")
        val viewModel = createRotationViewModel(repo, store, mutations)
        runCurrent()

        viewModel.rotateActiveItem()
        viewModel.flushPendingRotation()
        runCurrent()

        assertEquals(1, mutations.updateCalls)
        assertEquals("cloud_c1b", viewModel.uiState.value.activeMediaId)

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    @Test
    fun `a failed rotate drops the optimistic turn`() = testScope.runTest {
        val repo = rotationRepository()
        val store = TimelineProjectionStore(repo, backgroundScope, dispatchers)
        runCurrent()
        val mutations = FakeRotationMutationRepository(newCloudId = null)
        val viewModel = createRotationViewModel(repo, store, mutations)
        runCurrent()

        viewModel.rotateActiveItem()
        viewModel.flushPendingRotation()
        runCurrent()

        assertEquals(1, mutations.updateCalls)
        assertEquals("cloud_c1", viewModel.uiState.value.activeMediaId)
        assertTrue(viewModel.uiState.value.pendingRotationById.isEmpty())
        assertTrue(viewModel.uiState.value.mutatingIds.isEmpty())

        viewModel.viewModelScope.cancel()
        store.stop()
    }

    private suspend fun rotationRepository(): FakeMediaRepository {
        val repo = FakeMediaRepository()
        repo.upsertAll(
            listOf(
                MediaEntity(
                    id = "cloud_c1",
                    cloudId = "c1",
                    ratio = 2f,
                    sortTimestamp = 1000L,
                    yearMonth = 202611,
                    syncStatus = SyncStatus.CLOUD_ONLY,
                ),
            ),
        )
        return repo
    }

    private fun TestScope.createRotationViewModel(
        repo: FakeMediaRepository,
        store: TimelineProjectionStore,
        mutations: FakeRotationMutationRepository,
    ): MediaViewerViewModel = MediaViewerViewModel(
        initialMediaId = "cloud_c1",
        timelineProjectionStore = store,
        mediaRepository = repo,
        mediaDetailsRepository = detailsRepository,
        fullResFileCache = FakeFullResFileStore(),
        localMediaInfoReader = FakeLocalMediaInfoSource(),
        mediaFileSharer = FakeMediaFileSharer(),
        dispatchers = dispatchers,
        mediaMutationRepository = mutations,
        rotateMediaUseCase = RotateMediaUseCase(
            mediaRepository = repo,
            mediaMutationRepository = mutations,
            localFileOverwriteHelper = FakeRotationOverwriteHelper(),
            fullResFileCache = FakeFullResFileStore(),
        ),
        appScope = backgroundScope,
    )
}

private class FakeMediaDetailsRepository(
    private val items: Map<String, MediaDetails> = emptyMap(),
    private val delayMs: Long = 0L,
) : MediaDetailsRepository {
    val requested = mutableListOf<String>()

    override fun peek(cloudId: String): MediaDetails? = if (delayMs == 0L) items[cloudId] else null

    override suspend fun get(cloudId: String): MediaDetails? {
        requested += cloudId
        if (delayMs > 0L) delay(delayMs)
        return items[cloudId]
    }

    override fun evict(cloudIds: Collection<String>) = Unit

    override fun clear() = Unit
}

private class FakeFullResFileStore : FullResFileStore {
    val requestedCloudIds = mutableListOf<String>()

    override fun peek(cloudId: String): File? = null

    override suspend fun get(cloudId: String, fileName: String?): File? {
        requestedCloudIds += cloudId
        return null
    }

    override suspend fun evict(cloudId: String) = Unit

    override suspend fun clear() = Unit
}

private class FakeLocalMediaInfoSource(
    private val response: LocalMediaInfo? = null,
) : LocalMediaInfoSource {
    val requestedUris = mutableListOf<String>()

    override suspend fun read(contentUri: String, isVideo: Boolean): LocalMediaInfo? {
        requestedUris += contentUri
        return response
    }
}

private class FakeMediaFileSharer : MediaFileSharer {
    override suspend fun shareIntent(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
    ): Intent? = null

    override suspend fun copyToClipboard(
        item: TimelineMediaProjection,
        details: MediaDetails?,
        localInfo: LocalMediaInfo?,
    ): Boolean = false
}

private class FakeRotationMutationRepository(private val newCloudId: String?) : MediaMutationRepository {
    var updateCalls = 0

    override suspend fun updateItem(
        cloudId: String,
        request: UpdateMediaItemRequestDto,
    ): Result<String> {
        updateCalls++
        return newCloudId?.let { Result.success(it) } ?: Result.failure(IOException("offline"))
    }

    override suspend fun reprocess(cloudId: String): Result<Unit> = Result.success(Unit)
}

private class FakeRotationOverwriteHelper : LocalFileOverwriteHelper {
    override suspend fun writeOrientation(contentUri: String, orientation: Int): Long? = 100L

    override suspend fun overwriteFromCloud(
        cloudId: String,
        contentUri: String,
        fileName: String?,
    ): Boolean = true
}
