package dev.ruurd.photos.core.image

import kotlinx.collections.immutable.ImmutableMap
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.toPersistentMap
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * Per-item cache-busting revision counter bumped after in-place mutations (`SYNCED` / `LOCAL_ONLY`
 * rotations) so `TimelineTile` reloads even when `id`, `localContentUri`, and `widthDp` stay
 * identical (e.g. square photos or 180-degree turns).
 */
class MediaImageRevisions {
    private val _revisions = MutableStateFlow<ImmutableMap<String, Int>>(persistentMapOf())
    val revisions: StateFlow<ImmutableMap<String, Int>> = _revisions.asStateFlow()

    fun bump(id: String) {
        _revisions.update { current ->
            (current + (id to ((current[id] ?: 0) + 1))).toPersistentMap()
        }
    }

    fun clear() {
        _revisions.value = persistentMapOf()
    }
}
