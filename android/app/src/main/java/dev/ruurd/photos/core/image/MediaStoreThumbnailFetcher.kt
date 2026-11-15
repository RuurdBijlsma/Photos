package dev.ruurd.photos.core.image

import android.net.Uri
import android.os.CancellationSignal
import android.util.Size
import coil3.ImageLoader
import coil3.asImage
import coil3.decode.DataSource
import coil3.fetch.FetchResult
import coil3.fetch.Fetcher
import coil3.fetch.ImageFetchResult
import coil3.key.Keyer
import coil3.request.Options
import coil3.size.Dimension
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.job
import kotlinx.coroutines.withContext
import java.io.IOException
import kotlin.coroutines.coroutineContext

/**
 * Grid-only wrapper around a `MediaStore` URI. Using a dedicated type keeps the fast
 * [android.content.ContentResolver.loadThumbnail] path scoped to timeline tiles: the media viewer
 * keeps passing the raw `Uri` so Coil subsamples the full-resolution file for pinch-to-zoom.
 *
 * @param cloudFallbackUrl thumbnail to use when the local URI grant is gone (optional: the caller
 * knows the server URL and whether pre-generated thumbnails exist, the fetcher does not).
 */
data class MediaStoreThumbnail(
    val uri: Uri,
    val cloudId: String? = null,
    val cloudFallbackUrl: String? = null
)

class MediaStoreThumbnailFetcher(
    private val data: MediaStoreThumbnail,
    private val options: Options,
    private val imageLoader: ImageLoader
) : Fetcher {

    override suspend fun fetch(): FetchResult? = try {
        loadThumbnail()
    } catch (e: SecurityException) {
        // Temporary grant revoked (e.g. partial access after a restart): fall back to the server.
        fetchModel(data.cloudFallbackUrl)
    } catch (e: IOException) {
        // No MediaStore thumbnail yet; let Coil decode the file subsampled to the requested size.
        // shortcut: videos have no decoder in this module, so their tile stays empty until loadThumbnail works.
        fetchModel(data.uri.toString())
    }

    private suspend fun loadThumbnail(): FetchResult {
        val width = (options.size.width as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX
        val height = (options.size.height as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX
        val signal = CancellationSignal()

        val bitmap = withContext(Dispatchers.IO) {
            coroutineContext.job.invokeOnCompletion { signal.cancel() }
            options.context.contentResolver.loadThumbnail(data.uri, Size(width, height), signal)
        }

        return ImageFetchResult(bitmap.asImage(), isSampled = true, dataSource = DataSource.DISK)
    }

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
