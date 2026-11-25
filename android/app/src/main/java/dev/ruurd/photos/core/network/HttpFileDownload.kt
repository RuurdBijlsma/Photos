package dev.ruurd.photos.core.network

import dev.ruurd.photos.domain.model.MediaFileNames
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException
import java.io.OutputStream

internal data class DownloadedMediaMeta(
    val fileName: String,
    val mimeType: String?,
)

/**
 * Streams `GET /api/photos/{cloudId}/download` into [sink], resolving the best filename and MIME
 * type from caller hints, `Content-Disposition`, and fallback rules. Cancellation aborts the
 * OkHttp socket immediately; non-cancellation network/IO failures return `null`.
 */
internal suspend fun OkHttpClient.downloadMediaFile(
    serverUrl: String,
    cloudId: String,
    isVideo: Boolean,
    knownFileName: String? = null,
    knownMimeType: String? = null,
    sink: (meta: DownloadedMediaMeta, copyBody: (OutputStream) -> Unit) -> File?,
): File? {
    val job = currentCoroutineContext().job
    val request = Request.Builder()
        .url("$serverUrl/api/photos/$cloudId/download")
        .build()

    return try {
        newCall(request).await().use { response ->
            if (!response.isSuccessful) return null
            val responseMime = knownMimeType
                ?: response.header("Content-Type")?.substringBefore(';')?.trim()
            val fileName = knownFileName?.takeIf { it.isNotBlank() }
                ?: MediaFileNames.parseContentDispositionFilename(
                    response.header("Content-Disposition"),
                )
                ?: MediaFileNames.fallbackName(
                    cloudId = cloudId,
                    isVideo = isVideo,
                    mimeType = responseMime,
                )
            val meta = DownloadedMediaMeta(fileName = fileName, mimeType = responseMime)
            sink(meta) { output ->
                val buffer = ByteArray(STREAM_BUFFER_BYTES)
                response.body.byteStream().use { input ->
                    while (true) {
                        job.ensureActive()
                        val read = input.read(buffer)
                        if (read < 0) break
                        output.write(buffer, 0, read)
                    }
                }
            }
        }
    } catch (e: CancellationException) {
        throw e
    } catch (_: IOException) {
        null
    } catch (_: Exception) {
        null
    }
}

private const val STREAM_BUFFER_BYTES = 16 * 1024