package dev.ruurd.photos.core.image

import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.CancellationSignal
import android.os.OperationCanceledException
import android.util.Size
import androidx.compose.runtime.Immutable
import coil3.ImageLoader
import coil3.asImage
import coil3.decode.DataSource
import coil3.fetch.FetchResult
import coil3.fetch.Fetcher
import coil3.fetch.ImageFetchResult
import coil3.key.Keyer
import coil3.request.Options
import coil3.size.Dimension
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.withContext
import java.io.FileNotFoundException
import java.io.IOException
import kotlin.coroutines.coroutineContext

/**
 * Grid-only wrapper around a `MediaStore` URI. Using a dedicated type keeps the fast
 * [android.content.ContentResolver.loadThumbnail] path scoped to timeline tiles: the media viewer
 * keeps passing the raw `Uri` so Coil subsamples the full-resolution file for pinch-to-zoom.
 *
 * @param cloudFallbackUrl thumbnail to use when the local URI grant is gone or the local file is missing.
 */
@Immutable
data class MediaStoreThumbnail(
    val uri: Uri,
    val cloudId: String? = null,
    val cloudFallbackUrl: String? = null,
    val isVideo: Boolean = false
)

class MediaStoreThumbnailFetcher(
    private val data: MediaStoreThumbnail,
    private val options: Options,
    private val imageLoader: ImageLoader
) : Fetcher {

    override suspend fun fetch(): FetchResult? = try {
        loadThumbnail()
    } catch (_: SecurityException) {
        // Temporary grant revoked (e.g. partial access after a restart): fall back to the server.
        fetchModel(data.cloudFallbackUrl)
    } catch (_: FileNotFoundException) {
        // Local file was removed before the next scan reconciled Room: fall back to the server.
        fetchModel(data.cloudFallbackUrl)
    } catch (_: IOException) {
        // No MediaStore thumbnail yet: decode subsampled locally, falling back to cloud if needed.
        fetchLocalFallbackOrCloud()
    }

    private suspend fun loadThumbnail(): FetchResult {
        val width = requestedDimensionPx(options.size.width)
        val height = requestedDimensionPx(options.size.height)
        val signal = CancellationSignal()

        val bitmap = try {
            withContext(Dispatchers.IO) {
                coroutineContext.job.invokeOnCompletion { signal.cancel() }
                options.context.contentResolver.loadThumbnail(data.uri, Size(width, height), signal)
            }
        } catch (e: OperationCanceledException) {
            currentCoroutineContext().ensureActive()
            throw e
        }

        return ImageFetchResult(bitmap.asImage(), isSampled = true, dataSource = DataSource.DISK)
    }

    private suspend fun fetchLocalFallbackOrCloud(): FetchResult? {
        val isVideo = data.isVideo ||
                options.context.contentResolver.getType(data.uri)?.startsWith("video/") == true

        if (isVideo) {
            loadVideoFrame()?.let { return it }
            return fetchModel(data.cloudFallbackUrl)
        }

        return try {
            fetchModel(data.uri.toString()) ?: fetchModel(data.cloudFallbackUrl)
        } catch (_: IOException) {
            fetchModel(data.cloudFallbackUrl)
        }
    }

    private suspend fun loadVideoFrame(): FetchResult? {
        val width = requestedDimensionPx(options.size.width)
        val height = requestedDimensionPx(options.size.height)

        return withContext(Dispatchers.IO) {
            try {
                MediaMetadataRetriever().use { retriever ->
                    retriever.setDataSource(options.context, data.uri)
                    val frame = retriever.getScaledFrameAtTime(
                        -1L,
                        MediaMetadataRetriever.OPTION_CLOSEST_SYNC,
                        width,
                        height
                    ) ?: return@withContext null

                    ImageFetchResult(
                        frame.asImage(),
                        isSampled = true,
                        dataSource = DataSource.DISK
                    )
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                null
            }
        }
    }

    private fun requestedDimensionPx(dimension: Dimension): Int =
        ((dimension as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX).coerceAtLeast(1)

    /** Delegates to the regular component pipeline (content URI fetcher or network fetcher). */
    private suspend fun fetchModel(model: Any?): FetchResult? {
        if (model == null) return null
        val mapped = imageLoader.components.map(model, options)
        val fetcher = imageLoader.components.newFetcher(mapped, options, imageLoader)?.first ?: return null
        return fetcher.fetch()
    }

    class Factory : Fetcher.Factory<MediaStoreThumbnail> {
        override fun create(
            data: MediaStoreThumbnail,
            options: Options,
            imageLoader: ImageLoader
        ): Fetcher = MediaStoreThumbnailFetcher(data, options, imageLoader)
    }

    class ThumbnailKeyer : Keyer<MediaStoreThumbnail> {
        override fun key(data: MediaStoreThumbnail, options: Options): String =
            "mediastore:${data.uri}"
    }

    private companion object {
        const val DEFAULT_SIZE_PX = 480
    }
}