package dev.ruurd.photos.core.image

import android.content.Context
import androidx.collection.LruCache
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.painter.Painter
import coil3.ImageLoader
import coil3.compose.asPainter
import coil3.memory.MemoryCache

/**
 * O(1) MemoryCache.Key registry capturing keys from onSuccess in TimelineTile and ZoomableAsyncImage.
 * Avoids scanning imageLoader.memoryCache?.keys under lock during viewer swipes.
 */
class MemoryCacheKeyRegistry(maxSize: Int = 256) {
    private val cache = LruCache<String, MemoryCache.Key>(maxSize)

    @Synchronized
    fun put(key: String, memoryCacheKey: MemoryCache.Key) {
        cache.put(key, memoryCacheKey)
    }

    @Synchronized
    fun get(key: String): MemoryCache.Key? = cache.get(key)

    /** The decoded bitmap for [key] if it is still in the memory cache, for instant hero/preview paints. */
    fun peekPainter(imageLoader: ImageLoader, context: Context, key: String): Painter? = get(key)
        ?.let { memoryCacheKey -> imageLoader.memoryCache?.get(memoryCacheKey)?.image }
        ?.asPainter(context)

    @Synchronized
    fun remove(key: String): MemoryCache.Key? = cache.remove(key)

    @Synchronized
    fun clear() {
        cache.evictAll()
    }
}

/** Provided once by the app root; timeline tiles record keys and the viewer reads them. */
val LocalMemoryCacheKeyRegistry = staticCompositionLocalOf<MemoryCacheKeyRegistry> {
    error("No MemoryCacheKeyRegistry provided")
}
