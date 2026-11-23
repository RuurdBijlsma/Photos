package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.domain.model.MediaDetails

/**
 * In-memory cache of `GET /api/photos/{cloudId}/item` responses. Small (only the viewed item and
 * its neighbors are ever requested), so an LRU keyed by `cloudId` is enough.
 */
interface MediaDetailsRepository {
    /** Cached entry only, for synchronous frame-1 reads. */
    fun peek(cloudId: String): MediaDetails?

    /** Cached entry, else a network fetch. `null` while offline or when the server answers non-2xx. */
    suspend fun get(cloudId: String): MediaDetails?

    fun evict(cloudIds: Collection<String>)

    fun clear()
}
