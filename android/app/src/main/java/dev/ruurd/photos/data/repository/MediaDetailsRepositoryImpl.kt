package dev.ruurd.photos.data.repository

import androidx.collection.LruCache
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.SingleFlightGroup
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.dto.CameraSettingsDto
import dev.ruurd.photos.core.network.dto.FullMediaItemDto
import dev.ruurd.photos.core.network.dto.MediaItemWithAlbumsDto
import dev.ruurd.photos.domain.model.AlbumRef
import dev.ruurd.photos.domain.model.CameraDetails
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.PlaceDetails
import dev.ruurd.photos.domain.model.WeatherDetails
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import java.io.IOException

class MediaDetailsRepositoryImpl(
    private val apiService: PhotosApiService,
    scope: CoroutineScope,
    dispatchers: AppDispatchers,
) : MediaDetailsRepository {

    private val cache = LruCache<String, MediaDetails>(MAX_CACHED_ITEMS)
    private val fetches = SingleFlightGroup<String, MediaDetails?>(scope, dispatchers.io)

    override fun peek(cloudId: String): MediaDetails? = cache.get(cloudId)

    override suspend fun get(cloudId: String): MediaDetails? {
        peek(cloudId)?.let { return it }
        val generation = fetches.generation
        return fetches.load(cloudId) { id -> fetch(id, generation) }
    }

    override fun evict(cloudIds: Collection<String>) {
        cloudIds.forEach { id ->
            fetches.cancel(id)
            cache.remove(id)
        }
    }

    override fun clear() {
        fetches.clear()
        cache.evictAll()
    }

    private suspend fun fetch(cloudId: String, expectedGeneration: Long): MediaDetails? = try {
        val response = apiService.getMediaItem(cloudId)
        val body = response.body()
        if (response.isSuccessful && body != null) {
            body.toDomain(cloudId).also {
                currentCoroutineContext().ensureActive()
                if (fetches.generation == expectedGeneration) {
                    cache.put(cloudId, it)
                }
            }
        } else {
            null
        }
    } catch (e: CancellationException) {
        throw e
    } catch (_: IOException) {
        null
    } catch (_: Exception) {
        null
    }

    private companion object {
        const val MAX_CACHED_ITEMS = 64
    }
}

internal fun MediaItemWithAlbumsDto.toDomain(cloudId: String): MediaDetails {
    val item = mediaItem
    return MediaDetails(
        cloudId = cloudId,
        fileName = item.filename.ifBlank { null },
        mimeType = item.mediaFeatures?.mimeType,
        sizeBytes = item.mediaFeatures?.sizeBytes,
        width = item.width?.takeIf { it > 0 },
        height = item.height?.takeIf { it > 0 },
        isVideo = item.isVideo,
        durationMs = item.durationMs,
        takenAtLocal = item.takenAtLocal,
        timezoneOffsetSeconds = item.timezoneOffsetSeconds,
        timezoneName = item.timezoneName,
        missingSince = item.missingSince,
        userCaption = item.userCaption?.takeIf { it.isNotBlank() },
        orientation = item.orientation,
        usePanoramaViewer = item.usePanoramaViewer,
        camera = item.cameraSettings?.toDomain(),
        weather = item.weather?.let { WeatherDetails(it.temperature, it.condition, it.windSpeed, it.isDaytime) },
        place = item.toPlace(),
        albums = albums.mapNotNull { ref ->
            ref.name.takeIf { it.isNotBlank() }?.let { AlbumRef(id = ref.id, name = it) }
        },
    )
}

private fun FullMediaItemDto.toPlace(): PlaceDetails? {
    val gps = gps ?: return null
    return PlaceDetails(
        latitude = gps.latitude,
        longitude = gps.longitude,
        name = gps.location?.name?.takeIf { it.isNotBlank() },
        admin1 = gps.location?.admin1?.takeIf { it.isNotBlank() },
        countryName = gps.location?.countryName?.takeIf { it.isNotBlank() && it != "N/A" },
    )
}

private fun CameraSettingsDto.toDomain(): CameraDetails? {
    val details = CameraDetails(
        make = cameraMake,
        model = cameraModel,
        lensMake = lensMake,
        lensModel = lensModel,
        exposureTime = exposureTime,
        aperture = aperture,
        iso = iso,
        focalLength = focalLength,
        focalLengthIn35mm = focalLengthIn35mm,
        exposureCompensation = exposureCompensation,
    )
    return details.takeUnless { it.isEmpty }
}
