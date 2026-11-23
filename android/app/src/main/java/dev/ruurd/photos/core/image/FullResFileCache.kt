package dev.ruurd.photos.core.image

import android.content.Context
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException
import java.util.concurrent.ConcurrentHashMap

/**
 * `cacheDir/full_res_cache/` — a ~500 MB LRU of `/api/photos/{cloudId}/download` originals kept out
 * of Coil's 1 GB `image_cache` so 20–50 MB files never evict the grid thumbnails. Share and Copy
 * reuse the same file instead of downloading twice.
 */
interface FullResFileStore {
    /** Cached original for this item, without touching the network. */
    fun peek(cloudId: String): File?

    /** Cached original, else downloads it. Concurrent callers for one item share one download. */
    suspend fun get(cloudId: String, fileName: String): File?

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

    private val inFlight = ConcurrentHashMap<String, Deferred<File?>>()

    override fun peek(cloudId: String): File? = directoryFor(cloudId)
        .listFiles()
        ?.firstOrNull { it.isFile && it.length() > 0L }

    override suspend fun get(cloudId: String, fileName: String): File? {
        peek(cloudId)?.let { cached ->
            cached.setLastModified(System.currentTimeMillis())
            return cached
        }
        val deferred = inFlight.computeIfAbsent(cloudId) { id ->
            scope.async(dispatchers.io) { download(id, fileName) }
                .also { created -> created.invokeOnCompletion { inFlight.remove(id, created) } }
        }
        return deferred.await()
    }

    override suspend fun evict(cloudId: String) {
        withContext(dispatchers.io) {
            directoryFor(cloudId).deleteRecursively()
            Unit
        }
    }

    override suspend fun clear() {
        withContext(dispatchers.io) {
            directory.deleteRecursively()
            Unit
        }
    }

    private suspend fun download(cloudId: String, fileName: String): File? = withContext(dispatchers.io) {
        val url = "${sessionManager.getServerUrl()}/api/photos/$cloudId/download"
        val targetDirectory = directoryFor(cloudId)
        val target = File(targetDirectory, MediaFileNames.sanitize(fileName, isVideo = false, mimeType = null))
        val temp = File(targetDirectory, "${target.name}$TEMP_SUFFIX")

        try {
            val request = Request.Builder().url(url).build()
            okHttpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@withContext null
                targetDirectory.mkdirs()
                response.body.byteStream().use { input ->
                    temp.outputStream().use { output -> input.copyTo(output) }
                }
            }
            // Never truncate anything with an empty body: a 0-byte "original" would replace a good
            // cached file with an unopenable one.
            if (temp.length() <= 0L) {
                temp.delete()
                return@withContext null
            }
            target.delete()
            if (!temp.renameTo(target)) {
                temp.delete()
                return@withContext null
            }
            target.setLastModified(System.currentTimeMillis())
            trim()
            target
        } catch (e: CancellationException) {
            throw e
        } catch (_: IOException) {
            temp.delete()
            null
        } catch (_: Exception) {
            temp.delete()
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

    private companion object {
        const val CACHE_DIR = "full_res_cache"
        const val TEMP_SUFFIX = ".part"
        const val DEFAULT_MAX_SIZE_BYTES = 500L * 1024 * 1024
    }
}
