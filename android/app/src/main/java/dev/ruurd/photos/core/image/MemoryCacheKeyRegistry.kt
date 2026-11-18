package dev.ruurd.photos.core.image

import androidx.collection.LruCache
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

    @Synchronized
    fun remove(key: String): MemoryCache.Key? = cache.remove(key)

    @Synchronized
    fun clear() {
        cache.evictAll()
    }
}
