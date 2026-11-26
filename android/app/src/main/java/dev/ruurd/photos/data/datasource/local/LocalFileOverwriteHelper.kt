package dev.ruurd.photos.data.datasource.local

import android.content.Context
import android.media.MediaScannerConnection
import android.net.Uri
import android.provider.MediaStore
import androidx.core.net.toUri
import androidx.exifinterface.media.ExifInterface
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.image.FullResFileStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import kotlin.coroutines.resume

/**
 * Writes a server-side change back into the phone's copy so local bytes, size and BLAKE3 stay
 * identical to the server. The caller is responsible for the `MediaStore.createWriteRequest`
 * consent, which has to be requested from the UI.
 */
interface LocalFileOverwriteHelper {
    /** Writes [orientation] into the local photo's EXIF; returns the new size, or `null` on failure. */
    suspend fun writeOrientation(contentUri: String, orientation: Int): Long?

    /** Replaces the local file with the (already rotated) server original. */
    suspend fun overwriteFromCloud(cloudId: String, contentUri: String, fileName: String?): Boolean
}

class MediaStoreFileOverwriteHelper(
    context: Context,
    private val fullResFileCache: FullResFileStore,
    private val dispatchers: AppDispatchers,
) : LocalFileOverwriteHelper {
    private val appContext = context.applicationContext
    private val resolver = appContext.contentResolver

    override suspend fun writeOrientation(contentUri: String, orientation: Int): Long? = withContext(dispatchers.io) {
        val uri = contentUri.toUri()
        try {
            resolver.openFileDescriptor(uri, "rw")?.use { pfd ->
                // The descriptor must stay open until saveAttributes() flushes.
                ExifInterface(pfd.fileDescriptor).apply {
                    setAttribute(ExifInterface.TAG_ORIENTATION, orientation.toString())
                    saveAttributes()
                }
            } ?: return@withContext null
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            return@withContext null
        }
        // MediaProvider only refreshes ORIENTATION/SIZE after the scanner sees the file change.
        awaitMediaScan(queryDataPath(uri) ?: return@withContext null)
        querySize(uri)
    }

    override suspend fun overwriteFromCloud(
        cloudId: String,
        contentUri: String,
        fileName: String?,
    ): Boolean = withContext(dispatchers.io) {
        val uri = contentUri.toUri()
        val source = fullResFileCache.get(cloudId, fileName) ?: return@withContext false
        if (!source.isFile || source.length() <= 0L) return@withContext false

        try {
            resolver.openOutputStream(uri, "wt")?.use { output ->
                source.inputStream().use { input -> input.copyTo(output) }
            } ?: return@withContext false
        } catch (e: CancellationException) {
            throw e
        } catch (_: IOException) {
            return@withContext false
        } catch (_: Exception) {
            return@withContext false
        }

        awaitMediaScan(queryDataPath(uri) ?: return@withContext false)
        val size = querySize(uri) ?: return@withContext false
        size > 0L
    }

    /** `DATA` is deprecated but still answers for the user's own media rows. */
    private fun queryDataPath(uri: Uri): String? = try {
        resolver.query(uri, arrayOf(MediaStore.MediaColumns.DATA), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0)?.takeIf { it.isNotBlank() } else null
        }
    } catch (_: Exception) {
        null
    }

    private fun querySize(uri: Uri): Long? = try {
        resolver.query(uri, arrayOf(MediaStore.MediaColumns.SIZE), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getLong(0).takeIf { it > 0L } else null
        }
    } catch (_: Exception) {
        null
    }

    /**
     * Suspends until `MediaStore` finished re-reading the file, so a scan running right after does not
     * overwrite the change we just made with the old row values.
     */
    private suspend fun awaitMediaScan(path: String) {
        if (!File(path).exists()) return
        suspendCancellableCoroutine { continuation ->
            MediaScannerConnection.scanFile(appContext, arrayOf(path), null) { _, _ ->
                if (continuation.isActive) continuation.resume(Unit)
            }
        }
    }
}
