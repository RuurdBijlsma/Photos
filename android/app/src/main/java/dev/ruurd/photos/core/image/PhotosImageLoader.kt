package dev.ruurd.photos.core.image

import android.content.Context
import coil3.ImageLoader
import coil3.disk.DiskCache
import coil3.memory.MemoryCache
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
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
                add(
                    OkHttpNetworkFetcherFactory(
                        callFactory = { okHttpClient }
                    )
                )
            }
            .memoryCache {
                MemoryCache.Builder()
                    .maxSizePercent(context, 0.25)
                    .build()
            }
            .diskCache {
                DiskCache.Builder()
                    .directory(cacheDir.toOkioPath())
                    .maxSizeBytes(500L * 1024 * 1024) // 500 MB LRU disk cache
                    .build()
            }
            .build()
    }
}
