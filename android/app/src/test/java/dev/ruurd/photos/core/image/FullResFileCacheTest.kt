
package dev.ruurd.photos.core.image

import dev.ruurd.photos.testing.TestAppDispatchers
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

@OptIn(ExperimentalCoroutinesApi::class)
class FullResFileCacheTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    private val testDispatcher = StandardTestDispatcher()
    private val dispatchers = TestAppDispatchers(testDispatcher)

    private fun buildCache(
        maxSizeBytes: Long = 1024L * 1024L,
        onRequest: (String) -> Response,
    ): FullResFileCache {
        val client = OkHttpClient.Builder()
            .addInterceptor { chain -> onRequest(chain.request().url.toString()) }
            .build()
        return FullResFileCache(
            directory = tempFolder.root.resolve("full_res_cache"),
            okHttpClient = client,
            serverUrlProvider = { "http://localhost:5272" },
            scope = kotlinx.coroutines.CoroutineScope(testDispatcher),
            dispatchers = dispatchers,
            maxSizeBytes = maxSizeBytes,
        )
    }

    private fun okImageResponse(
        bytes: ByteArray,
        contentDisposition: String? = """inline; filename="photo.jpg"""",
    ): Response = Response.Builder()
        .request(okhttp3.Request.Builder().url("http://localhost/download").build())
        .protocol(Protocol.HTTP_1_1)
        .code(200)
        .message("OK")
        .apply { contentDisposition?.let { header("Content-Disposition", it) } }
        .body(bytes.toResponseBody("image/jpeg".toMediaType()))
        .build()

    @Test
    fun `peek ignores incomplete part files and returns completed files`() = runTest(testDispatcher) {
        val cache = buildCache { okImageResponse(byteArrayOf(1, 2, 3)) }
        val dir = tempFolder.root.resolve("full_res_cache/c1").apply { mkdirs() }
        File(dir, "photo.jpg.part").writeBytes(byteArrayOf(9, 9, 9))

        assertNull(cache.peek("c1"))

        val downloaded = cache.get("c1")
        assertNotNull(downloaded)
        assertEquals("photo.jpg", downloaded?.name)
        assertEquals("photo.jpg", cache.peek("c1")?.name)
    }

    @Test
    fun `concurrent callers for one cloudId share a single download`() = runTest(testDispatcher) {
        val calls = AtomicInteger(0)
        val release = CompletableDeferred<Unit>()
        val cache = buildCache {
            calls.incrementAndGet()
            runBlocking { release.await() }
            okImageResponse(byteArrayOf(1, 2, 3, 4))
        }

        val first = async { cache.get("c1") }
        val second = async { cache.get("c1") }
        runCurrent()

        release.complete(Unit)
        val file1 = first.await()
        val file2 = second.await()

        assertEquals(1, calls.get())
        assertNotNull(file1)
        assertEquals(file1, file2)
    }

    @Test
    fun `trim keeps total within budget, spares part files, and removes empty directories`() = runTest(testDispatcher) {
        val cache = buildCache(maxSizeBytes = 10L) { url ->
            val id = url.substringAfter("/photos/").substringBefore("/download")
            okImageResponse(ByteArray(6) { 1 }, contentDisposition = """inline; filename="$id.jpg"""")
        }

        // Seed an active .part file in c_active that must not be deleted by trim.
        val activeDir = tempFolder.root.resolve("full_res_cache/c_active").apply { mkdirs() }
        val activePart = File(activeDir, "uploading.jpg.part").apply {
            writeBytes(ByteArray(20) { 2 })
            setLastModified(1L)
        }

        val oldFile = cache.get("c_old")!!
        oldFile.setLastModified(1_000L)

        val newFile = cache.get("c_new")!!
        assertTrue(newFile.exists())
        assertFalse(oldFile.exists())
        assertFalse(tempFolder.root.resolve("full_res_cache/c_old").exists())
        assertTrue(activePart.exists())
    }

    @Test
    fun `evict deletes cached file cleanly`() = runTest(testDispatcher) {
        val cache = buildCache { okImageResponse(byteArrayOf(1, 2, 3)) }
        val cached = cache.get("c1")
        assertNotNull(cached)
        assertTrue(cached!!.exists())

        cache.evict("c1")

        assertNull(cache.peek("c1"))
        assertFalse(cached.exists())
    }
}
