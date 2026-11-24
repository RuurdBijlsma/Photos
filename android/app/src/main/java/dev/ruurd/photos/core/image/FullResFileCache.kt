package dev.ruurd.photos.core.image

import android.content.Context
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.await
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

/**
 * `cacheDir/full_res_cache/` — a ~500 MB LRU of `/api/photos/{cloudId}/download` originals kept out
 * of Coil's 1 GB `image_cache` so 20–50 MB files never evict the grid thumbnails. Share and Copy
 * reuse the same file instead of downloading twice.
 */
interface FullResFileStore {
    /** Cached original for this item, without touching the network. */
    fun peek(cloudId: String): File?

    /** Cached original, else downloads it. Concurrent callers for one item share one download. */
    suspend fun get(cloudId: String, fileName: String? = null): File?

    suspend fun evict(cloudId: String)

    suspend fun clear()
}

class FullResFileCache(
    context: Context,
    private val okHttpClient: OkHttpClient,
    private val sessionManager: SessionManager,
    private val scope: CoroutineScope,
    private val dispatchers: AppDispatchers,
    private val maxSizeBytes: Long = DEFAULT_MAX_SIZE_BYTES,
) : FullResFileStore {
    private val directory: File = context.cacheDir.resolve(CACHE_DIR)
    private val generation = AtomicLong(0L)
    private val inFlight = ConcurrentHashMap<String, InFlightDownload>()

    override fun peek(cloudId: String): File? = directoryFor(cloudId)
        .listFiles()
        ?.firstOrNull { it.isFile && it.length() > 0L }

    override suspend fun get(cloudId: String, fileName: String?): File? {
        peek(cloudId)?.let { cached ->
            cached.setLastModified(System.currentTimeMillis())
            return cached
        }
        val expectedGen = generation.get()
        val entry = inFlight.compute(cloudId) { id, existing ->
            val active = existing?.takeIf {
                it.generation == expectedGen && !it.job.isCancelled && !it.job.isCompleted
            } ?: InFlightDownload(
                generation = expectedGen,
                job = scope.async(dispatchers.io, start = CoroutineStart.LAZY) {
                    download(id, fileName, expectedGen)
                },
            ).also { created ->
                created.job.invokeOnCompletion { inFlight.remove(id, created) }
            }
            active.waiters.incrementAndGet()
            active
        }!!

        return try {
            entry.job.await()
        } finally {
            if (entry.waiters.decrementAndGet() <= 0) {
                inFlight.remove(cloudId, entry)
                entry.job.cancel()
            }
        }
    }

    override suspend fun evict(cloudId: String) {
        inFlight.remove(cloudId)?.job?.cancel()
        withContext(dispatchers.io) {
            directoryFor(cloudId).deleteRecursively()
            Unit
        }
    }

    override suspend fun clear() {
        generation.incrementAndGet()
        val pending = inFlight.values.toList()
        inFlight.clear()
        pending.forEach { it.job.cancel() }
        withContext(dispatchers.io) {
            directory.deleteRecursively()
            Unit
        }
    }

    private suspend fun download(
        cloudId: String,
        fileName: String?,
        expectedGeneration: Long,
    ): File? = withContext(dispatchers.io) {
        val url = "${sessionManager.getServerUrl()}/api/photos/$cloudId/download"
        val targetDirectory = directoryFor(cloudId)
        var temp: File? = null

        try {
            val request = Request.Builder().url(url).build()
            val resolvedName = okHttpClient.newCall(request).await().use { response ->
                if (!response.isSuccessful) return@withContext null

                val name = fileName?.takeIf { it.isNotBlank() }
                    ?: MediaFileNames.parseContentDispositionFilename(
                        response.header("Content-Disposition"),
                    )
                    ?: MediaFileNames.fallbackName(
                        cloudId = cloudId,
                        isVideo = false,
                        mimeType = response.header("Content-Type")?.substringBefore(';')?.trim(),
                    )
                val sanitized = MediaFileNames.sanitize(name, isVideo = false, mimeType = null)

                targetDirectory.mkdirs()
                val partFile = File(targetDirectory, "$sanitized$TEMP_SUFFIX")
                temp = partFile

                val buffer = ByteArray(STREAM_BUFFER_BYTES)
                response.body.byteStream().use { input ->
                    partFile.outputStream().use { output ->
                        while (true) {
                            currentCoroutineContext().ensureActive()
                            val read = input.read(buffer)
                            if (read < 0) break
                            output.write(buffer, 0, read)
                        }
                    }
                }
                sanitized
            }

            val partFile = temp ?: return@withContext null
            if (generation.get() != expectedGeneration || partFile.length() <= 0L) {
                partFile.delete()
                return@withContext null
            }

            val target = File(targetDirectory, resolvedName)
            target.delete()
            if (!partFile.renameTo(target)) {
                partFile.delete()
                return@withContext null
            }
            target.setLastModified(System.currentTimeMillis())
            trim()
            target
        } catch (e: CancellationException) {
            temp?.delete()
            throw e
        } catch (_: IOException) {
            temp?.delete()
            null
        } catch (_: Exception) {
            temp?.delete()
            null
        }
    }

    /** Drops the least recently used originals until the directory fits [maxSizeBytes]. */
    private fun trim() {
        val files = directory.listFiles()
            ?.flatMap { item -> item.listFiles()?.filter { it.isFile } ?: emptyList() }
            ?.sortedBy { it.lastModified() }
            ?: return
        var total = files.sumOf { it.length() }
        for (file in files) {
            if (total <= maxSizeBytes) break
            val length = file.length()
            if (file.delete()) total -= length
        }
    }

    private fun directoryFor(cloudId: String): File = directory.resolve(cloudId)

    private class InFlightDownload(
        val generation: Long,
        val job: Deferred<File?>,
    ) {
        val waiters = AtomicInteger(0)
    }

    private companion object {
        const val CACHE_DIR = "full_res_cache"
        const val TEMP_SUFFIX = ".part"
        const val STREAM_BUFFER_BYTES = 16 * 1024
        const val DEFAULT_MAX_SIZE_BYTES = 500L * 1024 * 1024
    }
}
