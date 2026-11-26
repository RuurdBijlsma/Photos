package dev.ruurd.photos.domain.repository

import dev.ruurd.photos.core.network.dto.UpdateMediaItemRequestDto

/** Mutations on a server media item, keyed by the server's `cloudId`. */
interface MediaMutationRepository {

    /**
     * `PUT /api/photos/{cloudId}/item`. Returns the item's `cloudId`, which the server changes when
     * the orientation changed.
     */
    suspend fun updateItem(cloudId: String, request: UpdateMediaItemRequestDto): Result<String>

    suspend fun updateOrientation(cloudId: String, orientation: Int): Result<String> = updateItem(cloudId, UpdateMediaItemRequestDto(orientation = orientation))

    /** `POST /api/photos/{cloudId}/reprocess`; the `cloudId` is unchanged. */
    suspend fun reprocess(cloudId: String): Result<Unit>
}
