package dev.ruurd.photos.core.image

import android.content.Context
import coil3.ImageLoader
import coil3.bitmapFactoryMaxParallelism
import coil3.disk.DiskCache
import coil3.memory.MemoryCache
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import coil3.util.DebugLogger
import coil3.util.Logger
import okhttp3.OkHttpClient
import okio.Path.Companion.toOkioPath

object PhotosImageLoader {

    fun create(
        context: Context,
        okHttpClient: OkHttpClient
    ): ImageLoader {
        val cacheDir = context.cacheDir.resolve("image_cache")

        return ImageLoader.Builder(context)
            .components {
                add(MediaStoreThumbnailFetcher.Factory())
                add(MediaStoreThumbnailFetcher.ThumbnailKeyer())
                add(LibavifDecoder.Factory())
                add(
                    OkHttpNetworkFetcherFactory(
                        callFactory = { okHttpClient }
                    )
                )
            }
            .memoryCache {
                MemoryCache.Builder()
                    .maxSizePercent(context, 0.35)
                    .build()
            }
            .diskCache {
                DiskCache.Builder()
                    .directory(cacheDir.toOkioPath())
                    .maxSizeBytes(1L * 1024 * 1024 * 1024) // 1 GB LRU disk cache
                    .build()
            }
            .build()
    }
}