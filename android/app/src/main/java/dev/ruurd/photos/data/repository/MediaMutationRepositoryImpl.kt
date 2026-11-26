package dev.ruurd.photos.data.repository

import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.dto.UpdateMediaItemRequestDto
import dev.ruurd.photos.domain.repository.MediaMutationRepository
import kotlinx.coroutines.CancellationException
import java.io.IOException

class MediaMutationRepositoryImpl(
    private val apiService: PhotosApiService,
) : MediaMutationRepository {

    override suspend fun updateItem(
        cloudId: String,
        request: UpdateMediaItemRequestDto,
    ): Result<String> = try {
        val response = apiService.updateMediaItem(cloudId, request)
        val body = response.body()
        if (response.isSuccessful && body != null && body.mediaItemId.isNotBlank()) {
            Result.success(body.mediaItemId)
        } else {
            Result.failure(MutationException("HTTP ${response.code()}"))
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: IOException) {
        Result.failure(e)
    } catch (e: Exception) {
        Result.failure(e)
    }

    override suspend fun reprocess(cloudId: String): Result<Unit> = try {
        val response = apiService.reprocessMediaItem(cloudId)
        if (response.isSuccessful) {
            Result.success(Unit)
        } else {
            Result.failure(MutationException("HTTP ${response.code()}"))
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: IOException) {
        Result.failure(e)
    } catch (e: Exception) {
        Result.failure(e)
    }
}

class MutationException(message: String) : Exception(message)
