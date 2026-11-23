package dev.ruurd.photos.data.repository

import androidx.collection.LruCache
import dev.ruurd.photos.core.common.AppDispatchers
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
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import java.io.IOException
import java.util.concurrent.ConcurrentHashMap

class MediaDetailsRepositoryImpl(
    private val apiService: PhotosApiService,
    private val scope: CoroutineScope,
    private val dispatchers: AppDispatchers,
) : MediaDetailsRepository {

    private val cache = LruCache<String, MediaDetails>(MAX_CACHED_ITEMS)

    // Neighbors of the settled page ask for the same cloudId the pager itself asks for.
    private val inFlight = ConcurrentHashMap<String, Deferred<MediaDetails?>>()

    override fun peek(cloudId: String): MediaDetails? = cache.get(cloudId)

    override suspend fun get(cloudId: String): MediaDetails? {
        peek(cloudId)?.let { return it }
        val deferred = inFlight.computeIfAbsent(cloudId) { id ->
            scope.async(dispatchers.io) { fetch(id) }
                .also { created -> created.invokeOnCompletion { inFlight.remove(id, created) } }
        }
        return deferred.await()
    }

    override fun evict(cloudIds: Collection<String>) {
        cloudIds.forEach { cache.remove(it) }
    }

    override fun clear() {
        cache.evictAll()
    }

    private suspend fun fetch(cloudId: String): MediaDetails? = try {
        val response = apiService.getMediaItem(cloudId)
        val body = response.body()
        if (response.isSuccessful && body != null) {
            body.toDomain(cloudId).also { cache.put(cloudId, it) }
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
