package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.dto.FullMediaItemDto
import dev.ruurd.photos.core.network.dto.MediaItemWithAlbumsDto
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import dev.ruurd.photos.testing.TestAppDispatchers
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import okhttp3.RequestBody
import okhttp3.ResponseBody
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import retrofit2.Response

@OptIn(ExperimentalCoroutinesApi::class)
class MediaDetailsRepositoryImplTest {

    private val testDispatcher = StandardTestDispatcher()
    private val dispatchers = TestAppDispatchers(testDispatcher)

    private class FakeDetailsApiService : PhotosApiService {
        var calls = 0
        var gate: CompletableDeferred<Response<MediaItemWithAlbumsDto>>? = null
        var nextResponse: (String) -> Response<MediaItemWithAlbumsDto> = { id ->
            Response.success(MediaItemWithAlbumsDto(mediaItem = FullMediaItemDto(id = id, filename = "$id.jpg")))
        }

        override suspend fun getMediaItem(cloudId: String): Response<MediaItemWithAlbumsDto> {
            calls++
            return gate?.await() ?: nextResponse(cloudId)
        }

        override suspend fun healthCheck(): Response<String> = Response.success("OK")
        override suspend fun login(request: RequestBody): Response<ResponseBody> = Response.success(null)
        override suspend fun getMe(): Response<ResponseBody> = Response.success(null)
        override suspend fun getTimelineSync(since: Long?): Response<TimelineSyncResponse> = Response.success(TimelineSyncResponse.getDefaultInstance())
    }

    @Test
    fun `get caches responses and deduplicates concurrent callers`() = runTest(testDispatcher) {
        val gate = CompletableDeferred<Response<MediaItemWithAlbumsDto>>()
        val api = FakeDetailsApiService().apply { this.gate = gate }
        val repo = MediaDetailsRepositoryImpl(api, backgroundScope, dispatchers)

        val first = async { repo.get("c1") }
        val second = async { repo.get("c1") }
        runCurrent()

        assertEquals(1, api.calls)
        gate.complete(
            Response.success(MediaItemWithAlbumsDto(mediaItem = FullMediaItemDto(id = "c1", filename = "one.jpg"))),
        )

        assertEquals("one.jpg", first.await()?.fileName)
        assertEquals("one.jpg", second.await()?.fileName)
        assertEquals("one.jpg", repo.peek("c1")?.fileName)
        assertEquals("one.jpg", repo.get("c1")?.fileName)
        assertEquals(1, api.calls)
    }

    @Test
    fun `evict and clear cancel in-flight fetches without cancelling active callers`() = runTest(testDispatcher) {
        val gate = CompletableDeferred<Response<MediaItemWithAlbumsDto>>()
        val api = FakeDetailsApiService().apply { this.gate = gate }
        val repo = MediaDetailsRepositoryImpl(api, backgroundScope, dispatchers)

        val inFlight = async { repo.get("c1") }
        runCurrent()

        repo.evict(listOf("c1"))
        assertNull(inFlight.await())

        val second = async { repo.get("c2") }
        runCurrent()
        repo.clear()
        assertNull(second.await())

        // Subsequent fetches succeed normally after eviction/clear.
        api.gate = null
        assertEquals("c2.jpg", repo.get("c2")?.fileName)
    }

    @Test
    fun `non-2xx responses return null without caching`() = runTest(testDispatcher) {
        val api = FakeDetailsApiService().apply {
            nextResponse = { Response.error(404, "".toResponseBody()) }
        }
        val repo = MediaDetailsRepositoryImpl(api, backgroundScope, dispatchers)

        assertNull(repo.get("missing"))
        assertNull(repo.peek("missing"))
    }
}
