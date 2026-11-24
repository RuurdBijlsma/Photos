package dev.ruurd.photos.data.datasource.local

import android.content.Context
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.provider.MediaStore
import androidx.core.net.toUri
import androidx.exifinterface.media.ExifInterface
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.permissions.MediaPermissions
import dev.ruurd.photos.domain.model.CameraDetails
import dev.ruurd.photos.domain.model.LocalMediaInfo
import kotlinx.coroutines.withContext
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/**
 * Info-sheet fallback for items physically on the phone: `MediaStore` display metadata for both
 * types, plus original-file EXIF for photos. `setRequireOriginal` needs `ACCESS_MEDIA_LOCATION`, so
 * an unprivileged read falls back to the plain URI and (if that fails too) to MediaStore values.
 */
interface LocalMediaInfoSource {
    suspend fun read(contentUri: String, isVideo: Boolean): LocalMediaInfo?
}

class LocalMediaInfoReader(
    context: Context,
    private val dispatchers: AppDispatchers,
) : LocalMediaInfoSource {
    private val appContext = context.applicationContext
    private val resolver = appContext.contentResolver

    override suspend fun read(contentUri: String, isVideo: Boolean): LocalMediaInfo? = withContext(dispatchers.io) {
        val uri = contentUri.toUri()
        val fromStore = readMediaStore(uri, isVideo)
        val merged = if (isVideo) {
            fromStore?.withVideoMetadata(uri)
        } else {
            (fromStore ?: LocalMediaInfo()).withExif(uri)
        }
        merged?.takeUnless { it.isEmpty }
    }

    private fun readMediaStore(uri: Uri, isVideo: Boolean): LocalMediaInfo? {
        val projection = buildList {
            add(MediaStore.MediaColumns.DISPLAY_NAME)
            add(MediaStore.MediaColumns.MIME_TYPE)
            add(MediaStore.MediaColumns.SIZE)
            add(MediaStore.MediaColumns.WIDTH)
            add(MediaStore.MediaColumns.HEIGHT)
            if (!isVideo) add(COLUMN_ORIENTATION)
            add(MediaStore.MediaColumns.DATE_TAKEN)
            add(MediaStore.MediaColumns.DATE_MODIFIED)
            if (isVideo) add(MediaStore.Video.Media.DURATION)
        }.toTypedArray()

        val cursor = try {
            resolver.query(uri, projection, null, null, null)
        } catch (_: Exception) {
            null
        } ?: return null

        return cursor.use {
            if (!it.moveToFirst()) return null
            val rawWidth = it.intOrNull(MediaStore.MediaColumns.WIDTH)?.takeIf { size -> size > 0 }
            val rawHeight = it.intOrNull(MediaStore.MediaColumns.HEIGHT)?.takeIf { size -> size > 0 }
            val orientation = if (isVideo) 0 else it.intOrNull(COLUMN_ORIENTATION) ?: 0
            val (width, height) = if (orientation == 90 || orientation == 270) {
                rawHeight to rawWidth
            } else {
                rawWidth to rawHeight
            }
            LocalMediaInfo(
                fileName = it.stringOrNull(MediaStore.MediaColumns.DISPLAY_NAME),
                mimeType = it.stringOrNull(MediaStore.MediaColumns.MIME_TYPE),
                sizeBytes = it.longOrNull(MediaStore.MediaColumns.SIZE)?.takeIf { size -> size > 0L },
                width = width,
                height = height,
                dateTakenMs = it.longOrNull(MediaStore.MediaColumns.DATE_TAKEN)?.takeIf { value -> value > 0L }
                    ?: it.longOrNull(MediaStore.MediaColumns.DATE_MODIFIED)?.times(1000L),
                durationMs = if (isVideo) it.longOrNull(MediaStore.Video.Media.DURATION) else null,
            )
        }
    }

    /** MediaStore rows for some videos miss duration/size; the retriever reads them from the file. */
    private fun LocalMediaInfo.withVideoMetadata(uri: Uri): LocalMediaInfo {
        if (durationMs != null && width != null && height != null) return this
        val retriever = MediaMetadataRetriever()
        return try {
            retriever.setDataSource(appContext, uri)
            copy(
                durationMs = durationMs
                    ?: retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull(),
                width = width
                    ?: retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)?.toIntOrNull(),
                height = height
                    ?: retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)?.toIntOrNull(),
            )
        } catch (_: Exception) {
            this
        } finally {
            try {
                retriever.release()
            } catch (_: Exception) {
                // Nothing left to release.
            }
        }
    }

    private fun LocalMediaInfo.withExif(uri: Uri): LocalMediaInfo {
        val exif = openExif(uri) ?: return this
        val latLong = exif.latLong?.takeUnless { it.size >= 2 && it[0] == 0.0 && it[1] == 0.0 }
        val latitude = latLong?.getOrNull(0)
        val longitude = latLong?.getOrNull(1)
        return copy(
            dateTakenMs = dateTakenMs ?: (
                exif.getAttribute(ExifInterface.TAG_DATETIME_ORIGINAL)
                    ?: exif.getAttribute(ExifInterface.TAG_DATETIME)
                )?.let(::parseExifDateTime),
            camera = CameraDetails(
                make = exif.getAttribute(ExifInterface.TAG_MAKE),
                model = exif.getAttribute(ExifInterface.TAG_MODEL),
                lensMake = exif.getAttribute(ExifInterface.TAG_LENS_MAKE),
                lensModel = exif.getAttribute(ExifInterface.TAG_LENS_MODEL),
                exposureTime = exif.doubleAttribute(ExifInterface.TAG_EXPOSURE_TIME),
                aperture = exif.doubleAttribute(ExifInterface.TAG_F_NUMBER),
                iso = exif.getAttributeInt(ExifInterface.TAG_PHOTOGRAPHIC_SENSITIVITY, 0)
                    .takeIf { it > 0 },
                focalLength = exif.doubleAttribute(ExifInterface.TAG_FOCAL_LENGTH),
                focalLengthIn35mm = exif.doubleAttribute(ExifInterface.TAG_FOCAL_LENGTH_IN_35MM_FILM),
                exposureCompensation = exif.doubleAttribute(ExifInterface.TAG_EXPOSURE_BIAS_VALUE),
            ).takeUnless { it.isEmpty },
            latitude = latitude,
            longitude = longitude,
        )
    }

    private fun openExif(uri: Uri): ExifInterface? {
        val original = if (MediaPermissions.hasLocationAccess(appContext)) {
            try {
                MediaStore.setRequireOriginal(uri)
            } catch (_: Exception) {
                null
            }
        } else {
            null
        }
        for (candidate in listOfNotNull(original, uri)) {
            try {
                resolver.openInputStream(candidate)?.use { return ExifInterface(it) }
            } catch (_: Exception) {
                // Missing ACCESS_MEDIA_LOCATION or an unreadable URI: try the next candidate.
            }
        }
        return null
    }

    private fun ExifInterface.doubleAttribute(tag: String): Float? = if (getAttribute(tag) == null) null else getAttributeDouble(tag, 0.0).toFloat()

    private fun parseExifDateTime(value: String): Long? = try {
        LocalDateTime.parse(value, EXIF_DATE_TIME)
            .atZone(ZoneId.systemDefault())
            .toInstant()
            .toEpochMilli()
    } catch (_: Exception) {
        null
    }

    private fun android.database.Cursor.stringOrNull(column: String): String? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getString)

    private fun android.database.Cursor.longOrNull(column: String): Long? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getLong)

    private fun android.database.Cursor.intOrNull(column: String): Int? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getInt)

    private companion object {
        const val COLUMN_ORIENTATION = "orientation"
        val EXIF_DATE_TIME: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy:MM:dd HH:mm:ss")
    }
}

private val LocalMediaInfo.isEmpty: Boolean
    get() = fileName == null && mimeType == null && sizeBytes == null && width == null &&
        height == null && dateTakenMs == null && durationMs == null && camera == null &&
        latitude == null && longitude == null
