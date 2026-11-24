package dev.ruurd.photos.core.image

import android.content.Context
import android.net.Uri
import androidx.core.content.FileProvider
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.withContext
import java.io.File
import java.io.OutputStream

/**
 * `cacheDir/share_cache/{id}/{filename}` — files handed to other apps through [FileProvider], so
 * shares and clipboard copies always carry the real filename and extension, even for cloud-only
 * items or when the receiver has no `READ_MEDIA_IMAGES`.
 */
class ShareCache(
    context: Context,
    private val dispatchers: AppDispatchers,
) {
    private val appContext = context.applicationContext
    private val directory: File = appContext.cacheDir.resolve(CACHE_DIR)
    private val authority: String = "${appContext.packageName}.fileprovider"

    fun uriFor(file: File): Uri = FileProvider.getUriForFile(appContext, authority, file)

    /**
     * Any already staged file for this item. When [expectedFileName] is non-null, requires a
     * matching filename so an earlier fallback-named file doesn't shadow the real filename.
     */
    fun peek(id: String, expectedFileName: String? = null): File? {
        val candidates = directory.resolve(id)
            .listFiles()
            ?.filter { it.isFile && !it.name.endsWith(TEMP_SUFFIX) && it.length() > 0L }
            ?: return null
        return if (expectedFileName != null) {
            candidates.firstOrNull { it.name == expectedFileName }
        } else {
            candidates.firstOrNull()
        }
    }

    /**
     * Writes [write] to a `*.part` sibling and renames it in place, so a receiver never sees a
     * truncated file. Blocking IO: call from an IO dispatcher.
     */
    fun stageInto(
        id: String,
        fileName: String,
        isVideo: Boolean,
        mimeType: String?,
        write: (OutputStream) -> Unit,
    ): File? {
        val folder = directory.resolve(id).apply { mkdirs() }
        val sanitizedName = MediaFileNames.sanitize(fileName, isVideo, mimeType)
        val destination = File(folder, sanitizedName)
        val temp = File(folder, "$sanitizedName$TEMP_SUFFIX")
        return try {
            temp.outputStream().use(write)
            if (temp.length() <= 0L) {
                temp.delete()
                null
            } else {
                destination.delete()
                if (temp.renameTo(destination)) {
                    // Clean up older fallback-named files in the same item directory.
                    folder.listFiles()
                        ?.filter { it.isFile && it.name != sanitizedName && !it.name.endsWith(TEMP_SUFFIX) }
                        ?.forEach { it.delete() }
                    destination.setLastModified(System.currentTimeMillis())
                    destination
                } else {
                    temp.delete()
                    null
                }
            }
        } catch (_: Exception) {
            temp.delete()
            null
        }
    }

    suspend fun evict(id: String) = withContext(dispatchers.io) {
        directory.resolve(id).deleteRecursively()
        Unit
    }

    /** Drops files older than a day so shared originals do not accumulate in the cache. */
    suspend fun prune() = withContext(dispatchers.io) {
        val cutoff = System.currentTimeMillis() - MAX_AGE_MS
        directory.listFiles()?.forEach { folder ->
            folder.listFiles()
                ?.filter { it.isFile && it.lastModified() < cutoff }
                ?.forEach { it.delete() }
            if (folder.listFiles().isNullOrEmpty()) folder.delete()
        }
    }

    suspend fun clear() = withContext(dispatchers.io) {
        directory.deleteRecursively()
    }

    private companion object {
        const val CACHE_DIR = "share_cache"
        const val TEMP_SUFFIX = ".part"
        const val MAX_AGE_MS = 24L * 60 * 60 * 1000
    }
}