package dev.ruurd.photos.core.image

import coil3.memory.MemoryCache
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MemoryCacheKeyRegistryTest {

    @Test
    fun `put and get retrieves memory cache key`() {
        val registry = MemoryCacheKeyRegistry(maxSize = 10)
        val key = MemoryCache.Key("photo_1_key")

        registry.put("photo_1", key)

        assertEquals(key, registry.get("photo_1"))
        assertNull(registry.get("non_existent"))
    }

    @Test
    fun `remove deletes key`() {
        val registry = MemoryCacheKeyRegistry(maxSize = 10)
        val key = MemoryCache.Key("photo_1_key")
        registry.put("photo_1", key)

        val removed = registry.remove("photo_1")

        assertEquals(key, removed)
        assertNull(registry.get("photo_1"))
    }

    @Test
    fun `clear evicts all entries`() {
        val registry = MemoryCacheKeyRegistry(maxSize = 10)
        registry.put("photo_1", MemoryCache.Key("k1"))
        registry.put("photo_2", MemoryCache.Key("k2"))

        registry.clear()

        assertNull(registry.get("photo_1"))
        assertNull(registry.get("photo_2"))
    }

    @Test
    fun `lru eviction preserves most recent entries`() {
        val registry = MemoryCacheKeyRegistry(maxSize = 2)
        registry.put("photo_1", MemoryCache.Key("k1"))
        registry.put("photo_2", MemoryCache.Key("k2"))
        registry.put("photo_3", MemoryCache.Key("k3"))

        assertNull(registry.get("photo_1"))
        assertEquals(MemoryCache.Key("k2"), registry.get("photo_2"))
        assertEquals(MemoryCache.Key("k3"), registry.get("photo_3"))
    }
}
