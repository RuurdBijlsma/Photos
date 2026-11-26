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
import coil3.request.CachePolicy
import coil3.request.Options
import coil3.size.Dimension
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.withContext
import java.io.FileNotFoundException
import java.io.IOException
import kotlin.coroutines.coroutineContext

/**
 * Thumbnail wrapper around a `MediaStore` URI. Using a dedicated type keeps grid thumbnail loading
 * separate from Telephoto's raw `Uri` subsampling pipeline.
 *
 * @param cloudFallbackUrl thumbnail to use when the local URI grant is gone or the local file is missing.
 * @param bypassPlatformThumbnail skips `ContentResolver.loadThumbnail` (used for viewer still-photo
 * previews so `MediaProvider` cached/EXIF thumbnails never mismatch the primary image crop or quality).
 */
@Immutable
data class MediaStoreThumbnail(
    val uri: Uri,
    val cloudId: String? = null,
    val cloudFallbackUrl: String? = null,
    val isVideo: Boolean = false,
    val bypassPlatformThumbnail: Boolean = false,
)

class MediaStoreThumbnailFetcher(
    private val data: MediaStoreThumbnail,
    private val options: Options,
    private val imageLoader: ImageLoader,
    /** Caps concurrent Binder IPC; this fetcher returns decoded bitmaps, so it bypasses Coil's decoder cap. */
    private val ioLimiter: Semaphore,
) : Fetcher {

    override suspend fun fetch(): FetchResult? {
        // Skip MediaStore/decoder IPC while fast-scrubbing (Coil's memory cache still serves hits
        // before fetch() is invoked).
        if (options.diskCachePolicy == CachePolicy.READ_ONLY ||
            options.diskCachePolicy == CachePolicy.DISABLED
        ) {
            return null
        }

        if (data.bypassPlatformThumbnail && !data.isVideo) {
            return fetchLocalFallbackOrCloud()
        }

        return try {
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
    }

    // The permit is released before any cloud fallback so a slow network reply cannot stall the
    // MediaStore budget of the other tiles.
    private suspend fun loadThumbnail(): FetchResult = ioLimiter.withPermit {
        val width = requestedDimensionPx(options.size.width)
        val height = requestedDimensionPx(options.size.height)
        val signal = CancellationSignal()

        val bitmap = try {
            withContext(Dispatchers.IO) {
                val handle = coroutineContext.job.invokeOnCompletion { signal.cancel() }
                try {
                    options.context.contentResolver.loadThumbnail(
                        data.uri,
                        Size(width, height),
                        signal,
                    )
                } finally {
                    handle.dispose()
                }
            }
        } catch (e: OperationCanceledException) {
            currentCoroutineContext().ensureActive()
            throw e
        }

        ImageFetchResult(bitmap.asImage(), isSampled = true, dataSource = DataSource.DISK)
    }

    private suspend fun fetchLocalFallbackOrCloud(): FetchResult? = try {
        val isVideo = data.isVideo ||
            options.context.contentResolver.getType(data.uri)?.startsWith("video/") == true

        if (isVideo) {
            loadVideoFrame() ?: fetchModel(data.cloudFallbackUrl)
        } else {
            fetchModel(data.uri.toString()) ?: fetchModel(data.cloudFallbackUrl)
        }
    } catch (_: SecurityException) {
        fetchModel(data.cloudFallbackUrl)
    } catch (_: IOException) {
        fetchModel(data.cloudFallbackUrl)
    }

    private suspend fun loadVideoFrame(): FetchResult? {
        val width = requestedDimensionPx(options.size.width)
        val height = requestedDimensionPx(options.size.height)

        return ioLimiter.withPermit {
            withContext(Dispatchers.IO) {
                try {
                    MediaMetadataRetriever().use { retriever ->
                        retriever.setDataSource(options.context, data.uri)
                        val frame = retriever.getScaledFrameAtTime(
                            -1L,
                            MediaMetadataRetriever.OPTION_CLOSEST_SYNC,
                            width,
                            height,
                        ) ?: return@withContext null

                        ImageFetchResult(
                            frame.asImage(),
                            isSampled = true,
                            dataSource = DataSource.DISK,
                        )
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (_: Exception) {
                    null
                }
            }
        }
    }

    private fun requestedDimensionPx(dimension: Dimension): Int = ((dimension as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX).coerceAtLeast(1)

    /** Delegates to the regular component pipeline (content URI fetcher or network fetcher). */
    private suspend fun fetchModel(model: Any?): FetchResult? {
        if (model == null) return null
        val mapped = imageLoader.components.map(model, options)
        val fetcher = imageLoader.components.newFetcher(mapped, options, imageLoader)?.first ?: return null
        return fetcher.fetch()
    }

    class Factory(maxParallelism: Int = DEFAULT_MAX_PARALLELISM) : Fetcher.Factory<MediaStoreThumbnail> {
        private val ioLimiter = Semaphore(maxParallelism)

        override fun create(
            data: MediaStoreThumbnail,
            options: Options,
            imageLoader: ImageLoader,
        ): Fetcher = MediaStoreThumbnailFetcher(data, options, imageLoader, ioLimiter)
    }

    class ThumbnailKeyer : Keyer<MediaStoreThumbnail> {
        override fun key(data: MediaStoreThumbnail, options: Options): String {
            val width = (options.size.width as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX
            val height = (options.size.height as? Dimension.Pixels)?.px ?: DEFAULT_SIZE_PX
            return mediaStoreThumbnailCacheKey(
                uri = data.uri.toString(),
                width = width,
                height = height,
                bypassPlatformThumbnail = data.bypassPlatformThumbnail && !data.isVideo,
            )
        }
    }

    private companion object {
        const val DEFAULT_SIZE_PX = 480
        const val DEFAULT_MAX_PARALLELISM = 4
    }
}

/** Cache key of a local preview, shared with [MediaImageCacheEvictor] so both stay in sync. */
internal fun mediaStoreThumbnailCacheKey(
    uri: String,
    width: Int,
    height: Int,
    bypassPlatformThumbnail: Boolean,
): String {
    val prefix = if (bypassPlatformThumbnail) "mediastore_full" else "mediastore"
    return "$prefix:$uri?size=${width}x$height"
}
