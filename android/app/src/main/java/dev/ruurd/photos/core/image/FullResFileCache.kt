package dev.ruurd.photos.core.image

import android.content.Context
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.SingleFlightGroup
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.downloadMediaFile
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import java.io.File

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

class FullResFileCache internal constructor(
    private val directory: File,
    private val okHttpClient: OkHttpClient,
    private val serverUrlProvider: () -> String,
    scope: CoroutineScope,
    private val dispatchers: AppDispatchers,
    private val maxSizeBytes: Long = DEFAULT_MAX_SIZE_BYTES,
) : FullResFileStore {

    constructor(
        context: Context,
        okHttpClient: OkHttpClient,
        sessionManager: SessionManager,
        scope: CoroutineScope,
        dispatchers: AppDispatchers,
        maxSizeBytes: Long = DEFAULT_MAX_SIZE_BYTES,
    ) : this(
        directory = context.cacheDir.resolve(CACHE_DIR),
        okHttpClient = okHttpClient,
        serverUrlProvider = sessionManager::getServerUrl,
        scope = scope,
        dispatchers = dispatchers,
        maxSizeBytes = maxSizeBytes,
    )

    private val downloads = SingleFlightGroup<String, File?>(scope, dispatchers.io)
    private val trimLock = Any()

    override fun peek(cloudId: String): File? = directoryFor(cloudId)
        .listFiles()
        ?.firstOrNull { it.isFile && !it.name.endsWith(TEMP_SUFFIX) && it.length() > 0L }

    override suspend fun get(cloudId: String, fileName: String?): File? {
        peek(cloudId)?.let { cached ->
            cached.setLastModified(System.currentTimeMillis())
            return cached
        }
        val generation = downloads.generation
        return downloads.load(cloudId) { id -> download(id, fileName, generation) }
    }

    override suspend fun evict(cloudId: String) {
        downloads.cancel(cloudId)
        withContext(dispatchers.io) {
            synchronized(trimLock) {
                directoryFor(cloudId).deleteRecursively()
            }
            Unit
        }
    }

    override suspend fun clear() {
        downloads.clear()
        withContext(dispatchers.io) {
            synchronized(trimLock) {
                directory.deleteRecursively()
            }
            Unit
        }
    }

    private suspend fun download(
        cloudId: String,
        fileName: String?,
        expectedGeneration: Long,
    ): File? = withContext(dispatchers.io) {
        val job = currentCoroutineContext().job
        val targetDirectory = directoryFor(cloudId)
        var temp: File? = null

        try {
            okHttpClient.downloadMediaFile(
                serverUrl = serverUrlProvider(),
                cloudId = cloudId,
                isVideo = false,
                knownFileName = fileName,
            ) { meta, copyBody ->
                val sanitized = MediaFileNames.sanitize(
                    fileName = meta.fileName,
                    isVideo = false,
                    mimeType = meta.mimeType,
                )
                targetDirectory.mkdirs()
                val partFile = File(targetDirectory, "$sanitized$TEMP_SUFFIX").also { temp = it }

                try {
                    partFile.outputStream().use(copyBody)
                } catch (t: Throwable) {
                    partFile.delete()
                    throw t
                }

                synchronized(trimLock) {
                    job.ensureActive()
                    if (downloads.generation != expectedGeneration ||
                        !partFile.exists() ||
                        partFile.length() <= 0L
                    ) {
                        partFile.delete()
                        return@downloadMediaFile null
                    }

                    val target = File(targetDirectory, sanitized)
                    target.delete()
                    if (!partFile.renameTo(target)) {
                        partFile.delete()
                        return@downloadMediaFile null
                    }
                    target.setLastModified(System.currentTimeMillis())
                    trimLocked()
                    target
                }
            }
        } catch (e: CancellationException) {
            temp?.delete()
            throw e
        }
    }

    /** Drops the least recently used originals until the directory fits [maxSizeBytes]. */
    private fun trimLocked() {
        val files = directory.listFiles()
            ?.flatMap { item ->
                item.listFiles()?.filter { it.isFile && !it.name.endsWith(TEMP_SUFFIX) }
                    ?: emptyList()
            }
            ?.sortedBy { it.lastModified() }
            ?: return
        var total = files.sumOf { it.length() }
        for (file in files) {
            if (total <= maxSizeBytes) break
            val length = file.length()
            if (file.delete()) {
                total -= length
                val parent = file.parentFile
                if (parent != null && parent.listFiles().isNullOrEmpty()) {
                    parent.delete()
                }
            }
        }
    }

    private fun directoryFor(cloudId: String): File = directory.resolve(cloudId)

    private companion object {
        const val CACHE_DIR = "full_res_cache"
        const val TEMP_SUFFIX = ".part"
        const val DEFAULT_MAX_SIZE_BYTES = 500L * 1024 * 1024
    }
}