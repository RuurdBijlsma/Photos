package dev.ruurd.photos.core.image

import android.graphics.Bitmap
import coil3.ImageLoader
import coil3.asImage
import coil3.decode.DecodeResult
import coil3.decode.Decoder
import coil3.decode.ImageSource
import coil3.fetch.SourceFetchResult
import coil3.request.Options
import coil3.request.allowHardware
import coil3.request.maxBitmapSize
import coil3.size.Dimension
import coil3.size.Scale
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import okio.BufferedSource
import java.io.FileInputStream
import java.nio.ByteBuffer
import kotlin.math.min
import kotlin.math.roundToInt
import org.aomedia.avif.android.AvifDecoder as NativeAvifDecoder

/**
 * In-process AVIF decoder backed by `libavif` + `dav1d`.
 *
 * Android's platform `ImageDecoder` routes AVIF through `MediaCodec` over Binder IPC (~130-175 ms
 * per tile, serialized). Decoding still AVIFs in-process bypasses Binder/MediaCodec session setup,
 * while animated AVIF sequences (`avis`/`msf1`) return `null` from [Factory.create] to fall back
 * to the platform decoder.
 */
class LibavifDecoder(
    private val source: ImageSource,
    private val options: Options,
    private val parallelismLock: Semaphore,
    private val threads: Int
) : Decoder {

    override suspend fun decode(): DecodeResult? = parallelismLock.withPermit {
        currentCoroutineContext().ensureActive()

        val buffer = readToDirectByteBuffer(source)
        currentCoroutineContext().ensureActive()

        val info = NativeAvifDecoder.Info()
        buffer.rewind()
        if (!NativeAvifDecoder.getInfo(buffer, buffer.remaining(), info)) {
            return@withPermit null
        }
        if (info.width <= 0 || info.height <= 0) {
            return@withPermit null
        }

        val (dstWidth, dstHeight) = computeTargetDimensions(
            srcWidth = info.width,
            srcHeight = info.height,
            options = options
        )
        val isSampled = dstWidth < info.width || dstHeight < info.height
        val config = if (info.depth > 8) Bitmap.Config.RGBA_F16 else Bitmap.Config.ARGB_8888

        currentCoroutineContext().ensureActive()
        val bitmap = Bitmap.createBitmap(dstWidth, dstHeight, config)

        buffer.rewind()
        val decoded = NativeAvifDecoder.decode(buffer, buffer.remaining(), bitmap, threads)
        if (!decoded) {
            bitmap.recycle()
            return@withPermit null
        }

        DecodeResult(
            image = (if (options.allowHardware) bitmap.toHardwareBitmapOrSelf() else bitmap).asImage(),
            isSampled = isSampled
        )
    }

    /**
     * Uploads to a GPU texture here instead of letting `RenderThread` upload a software bitmap while
     * the next frames are already being drawn. Falls back to the software bitmap when the copy fails.
     */
    private fun Bitmap.toHardwareBitmapOrSelf(): Bitmap = try {
        copy(Bitmap.Config.HARDWARE, false)?.also { recycle() } ?: this
    } catch (_: Throwable) {
        this
    }

    private fun readToDirectByteBuffer(imageSource: ImageSource): ByteBuffer {
        val file = imageSource.fileOrNull()?.toFile()?.takeIf { it.isFile }
        if (file != null) {
            val length = file.length()
            if (length in 1L..Int.MAX_VALUE.toLong()) {
                val buffer = ByteBuffer.allocateDirect(length.toInt())
                FileInputStream(file).channel.use { channel ->
                    while (buffer.hasRemaining()) {
                        if (channel.read(buffer) == -1) break
                    }
                }
                buffer.flip()
                return buffer
            }
        }

        val bytes = imageSource.source().use { it.readByteArray() }
        return ByteBuffer.allocateDirect(bytes.size).apply {
            put(bytes)
            flip()
        }
    }

    private fun computeTargetDimensions(
        srcWidth: Int,
        srcHeight: Int,
        options: Options
    ): Pair<Int, Int> {
        val dstW = (options.size.width as? Dimension.Pixels)?.px
        val dstH = (options.size.height as? Dimension.Pixels)?.px

        var multiplier = when {
            dstW != null && dstH != null -> {
                val widthRatio = dstW.toDouble() / srcWidth
                val heightRatio = dstH.toDouble() / srcHeight
                when (options.scale) {
                    Scale.FILL -> maxOf(widthRatio, heightRatio)
                    Scale.FIT -> minOf(widthRatio, heightRatio)
                }
            }
            dstW != null -> dstW.toDouble() / srcWidth
            dstH != null -> dstH.toDouble() / srcHeight
            else -> 1.0
        }.coerceAtMost(1.0)

        val maxW = (options.maxBitmapSize.width as? Dimension.Pixels)?.px
        val maxH = (options.maxBitmapSize.height as? Dimension.Pixels)?.px
        if (maxW != null) {
            multiplier = min(multiplier, maxW.toDouble() / srcWidth)
        }
        if (maxH != null) {
            multiplier = min(multiplier, maxH.toDouble() / srcHeight)
        }

        val outW = (srcWidth * multiplier).roundToInt().coerceAtLeast(1)
        val outH = (srcHeight * multiplier).roundToInt().coerceAtLeast(1)
        return outW to outH
    }

    class Factory(
        maxParallelism: Int = DEFAULT_MAX_PARALLELISM,
        private val threads: Int = DEFAULT_THREADS_PER_DECODE
    ) : Decoder.Factory {

        private val parallelismLock = Semaphore(maxParallelism)

        override fun create(
            result: SourceFetchResult,
            options: Options,
            imageLoader: ImageLoader
        ): Decoder? {
            if (!isStillAvif(result.source.source())) return null
            return LibavifDecoder(
                source = result.source,
                options = options,
                parallelismLock = parallelismLock,
                threads = threads
            )
        }
    }

    private companion object {
        const val DEFAULT_MAX_PARALLELISM = 4
        const val DEFAULT_THREADS_PER_DECODE = 1

        const val MAX_FTYP_BYTES = 512L
        const val BOX_FTYP = 0x66747970 // "ftyp"
        const val BRAND_AVIF = 0x61766966 // "avif"
        const val BRAND_AVIS = 0x61766973 // "avis"
        const val BRAND_MSF1 = 0x6D736631 // "msf1"

        fun isStillAvif(source: BufferedSource): Boolean {
            val peek = source.peek()
            if (!peek.request(16L)) return false

            val boxSize = peek.readInt().toLong() and 0xFFFFFFFFL
            val boxType = peek.readInt()
            if (boxType != BOX_FTYP || boxSize < 16L || boxSize > MAX_FTYP_BYTES) return false
            if (!peek.request(boxSize - 8L)) return false

            val majorBrand = peek.readInt()
            if (majorBrand == BRAND_AVIS || majorBrand == BRAND_MSF1) return false

            var hasAvif = majorBrand == BRAND_AVIF
            peek.skip(4L) // minor_version

            var offset = 16L
            while (offset + 4L <= boxSize) {
                val brand = peek.readInt()
                if (brand == BRAND_AVIS || brand == BRAND_MSF1) return false
                if (brand == BRAND_AVIF) hasAvif = true
                offset += 4L
            }
            return hasAvif
        }
    }
}