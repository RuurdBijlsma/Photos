package dev.ruurd.photos.features.viewer.components

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CameraAlt
import androidx.compose.material.icons.outlined.CloudDone
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.CloudQueue
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Nightlight
import androidx.compose.material.icons.outlined.PhotoLibrary
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.net.toUri
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaInfoFormat
import dev.ruurd.photos.ui.theme.PhotosRadii

/**
 * Inline Media Info panel anchored directly beneath the active photo/video in shared vertical
 * coordinate space. Measured unbounded by the parent Layout so dragging upward continuously scrolls
 * the info content and pushes the photo smoothly off the top of the viewport.
 */
@Composable
fun MediaInfoPanel(
    details: MediaDetails?,
    localInfo: LocalMediaInfo?,
    item: TimelineMediaProjection?,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current

    val resolvedDateTime = remember(details, item, localInfo) {
        MediaInfoFormat.resolveLocalDateTime(details, item, localInfo)
    }
    val captureDateTime = remember(resolvedDateTime) {
        resolvedDateTime?.let { MediaInfoFormat.formatCompactDateTime(it) }
    }
    val timezone = remember(details) {
        MediaInfoFormat.timezoneLabel(details?.timezoneOffsetSeconds, details?.timezoneName)
    }
    val fileName = details?.fileName ?: localInfo?.fileName
    val width = details?.width ?: localInfo?.width
    val height = details?.height ?: localInfo?.height
    val megapixels = MediaInfoFormat.formatMegapixels(width, height)
    val resolution = MediaInfoFormat.formatResolution(width, height)
    val fileSize = MediaInfoFormat.formatSize(details?.sizeBytes ?: localInfo?.sizeBytes)
    val duration = MediaInfoFormat.formatDuration(details?.durationMs ?: localInfo?.durationMs)
    val mimeType = details?.mimeType ?: localInfo?.mimeType

    val camera = details?.camera ?: localInfo?.camera
    val cameraName = MediaInfoFormat.cameraName(camera)
    val lensName = MediaInfoFormat.lensName(camera)
    val aperture = MediaInfoFormat.apertureLabel(camera?.aperture)
    val shutter = MediaInfoFormat.exposureLabel(camera?.exposureTime)
    val focal = MediaInfoFormat.focalLabel(camera?.focalLength, camera?.focalLengthIn35mm)
    val iso = MediaInfoFormat.isoLabel(camera?.iso)
    val exposureBias = MediaInfoFormat.exposureCompensationLabel(camera?.exposureCompensation)
    val focal35mm = camera?.focalLengthIn35mm?.takeIf { it > 0f }?.let { "${it.toInt()} mm" }

    val primaryExifMetrics = listOfNotNull(
        aperture?.let { "Aperture" to it },
        shutter?.let { "Shutter" to it },
        focal?.let { "Focal" to it },
        iso?.let { "ISO" to it },
    )
    val secondaryExifMetrics = listOfNotNull(
        exposureBias?.let { "Exposure bias" to it },
        focal35mm?.let { "35mm equiv." to it },
    )

    val weather = MediaInfoFormat.weatherSummary(details?.weather)
    val windSummary = details?.weather?.windSpeed?.let { "Wind %.1f km/h".format(it) }

    val place = details?.place
    val latitude = place?.latitude ?: localInfo?.latitude
    val longitude = place?.longitude ?: localInfo?.longitude
    val placeLabel = MediaInfoFormat.locationLabel(place)
    val coordinates = MediaInfoFormat.coordinates(latitude, longitude)

    val albums = details?.albums.orEmpty()
    val specChips = listOfNotNull(megapixels, resolution, fileSize, duration, mimeType)

    val hasCameraHeader = cameraName != null || lensName != null
    val hasExifMetrics = primaryExifMetrics.isNotEmpty() || secondaryExifMetrics.isNotEmpty()
    val hasFileSpecs = fileName != null || specChips.isNotEmpty()

    val detailCardTypes = buildList {
        if (hasCameraHeader) add(DetailGroupCard.CAMERA_HEADER)
        if (hasExifMetrics) add(DetailGroupCard.EXIF_METRICS)
        if (hasFileSpecs) add(DetailGroupCard.FILE_SPECS)
    }

    val hasDetailsSection = item != null ||
            detailCardTypes.isNotEmpty() ||
            weather != null ||
            windSummary != null

    Surface(
        shape = RoundedCornerShape(topStart = PhotosRadii.Sheet, topEnd = PhotosRadii.Sheet),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        tonalElevation = 2.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = 20.dp)
                .padding(top = 10.dp, bottom = 36.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // Drag handle pill
            Box(
                modifier = Modifier.fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .size(width = 36.dp, height = 4.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.35f)),
                )
            }

            // Capture Date & Time + Timezone
            if (captureDateTime != null || timezone != null) {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    captureDateTime?.let { dateTimeLabel ->
                        Text(
                            text = dateTimeLabel,
                            style = MaterialTheme.typography.bodyLarge.copy(
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 15.sp,
                                lineHeight = 20.sp,
                                letterSpacing = 0.sp,
                            ),
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                    timezone?.let { tz ->
                        Text(
                            text = tz,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // User Caption
            details?.userCaption?.let { caption ->
                Surface(
                    shape = RoundedCornerShape(PhotosRadii.NestedCard),
                    color = MaterialTheme.colorScheme.surfaceContainer,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        text = caption,
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                    )
                }
            }

            // Albums Section
            if (albums.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    SectionTitle("Albums")
                    albums.forEach { album ->
                        AlbumPreviewRow(
                            title = album.name,
                            subtitle = "In album",
                        )
                    }
                }
            }

            // Location Section
            if ((latitude != null && longitude != null) || placeLabel != null) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    SectionTitle("Location")
                    Surface(
                        shape = RoundedCornerShape(20.dp),
                        color = MaterialTheme.colorScheme.surfaceContainerHigh,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(20.dp))
                            .then(
                                if (latitude != null && longitude != null) {
                                    Modifier.clickable { openMaps(context, latitude, longitude) }
                                } else {
                                    Modifier
                                },
                            ),
                    ) {
                        Row(
                            modifier = Modifier.padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(40.dp)
                                    .clip(CircleShape)
                                    .background(MaterialTheme.colorScheme.primaryContainer),
                                contentAlignment = Alignment.Center,
                            ) {
                                Icon(
                                    imageVector = Icons.Outlined.LocationOn,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onPrimaryContainer,
                                    modifier = Modifier.size(20.dp),
                                )
                            }
                            Spacer(modifier = Modifier.width(14.dp))
                            Column(
                                modifier = Modifier.weight(1f),
                                verticalArrangement = Arrangement.spacedBy(2.dp),
                            ) {
                                Text(
                                    text = placeLabel ?: "Pinned location",
                                    style = MaterialTheme.typography.titleSmall.copy(
                                        fontWeight = FontWeight.SemiBold,
                                    ),
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                                coordinates?.let { coords ->
                                    Text(
                                        text = coords,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                            if (latitude != null && longitude != null) {
                                TextButton(
                                    onClick = { openMaps(context, latitude, longitude) },
                                    contentPadding = PaddingValues(
                                        horizontal = 8.dp,
                                        vertical = 4.dp,
                                    ),
                                ) {
                                    Text("Open in Maps")
                                }
                            }
                        }
                    }
                }
            }

            // Details Section
            if (hasDetailsSection) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionTitle("Details")

                    if (item != null) {
                        SyncStatusCard(
                            syncStatus = item.syncStatus,
                            isOfflineDetailsMissing = details == null && item.cloudId != null,
                        )
                    }

                    if (detailCardTypes.isNotEmpty()) {
                        if (item != null) {
                            Spacer(modifier = Modifier.height(2.dp))
                        }
                        detailCardTypes.forEachIndexed { index, cardType ->
                            val shape = groupedCardShape(index, detailCardTypes.size)
                            when (cardType) {
                                DetailGroupCard.CAMERA_HEADER -> {
                                    Surface(
                                        shape = shape,
                                        color = MaterialTheme.colorScheme.surfaceContainerHigh,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Row(
                                            modifier = Modifier.padding(16.dp),
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Icon(
                                                imageVector = Icons.Outlined.CameraAlt,
                                                contentDescription = null,
                                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                                modifier = Modifier.size(22.dp),
                                            )
                                            Spacer(modifier = Modifier.width(14.dp))
                                            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                                cameraName?.let { name ->
                                                    Text(
                                                        text = name,
                                                        style = MaterialTheme.typography.titleSmall.copy(
                                                            fontWeight = FontWeight.SemiBold,
                                                        ),
                                                        color = MaterialTheme.colorScheme.onSurface,
                                                    )
                                                }
                                                lensName?.let { lens ->
                                                    Text(
                                                        text = lens,
                                                        style = MaterialTheme.typography.bodySmall,
                                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }

                                DetailGroupCard.EXIF_METRICS -> {
                                    Surface(
                                        shape = shape,
                                        color = MaterialTheme.colorScheme.surfaceContainerHigh,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Column(
                                            modifier = Modifier.padding(
                                                horizontal = 16.dp,
                                                vertical = 14.dp,
                                            ),
                                            verticalArrangement = Arrangement.spacedBy(12.dp),
                                        ) {
                                            if (primaryExifMetrics.isNotEmpty()) {
                                                Row(
                                                    modifier = Modifier.fillMaxWidth(),
                                                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                                                ) {
                                                    primaryExifMetrics.forEach { (label, value) ->
                                                        ExifMetric(
                                                            label = label,
                                                            value = value,
                                                            modifier = Modifier.weight(1f),
                                                        )
                                                    }
                                                }
                                            }
                                            if (primaryExifMetrics.isNotEmpty() &&
                                                secondaryExifMetrics.isNotEmpty()
                                            ) {
                                                HorizontalDivider(
                                                    color = MaterialTheme.colorScheme.outlineVariant.copy(
                                                        alpha = 0.4f,
                                                    ),
                                                )
                                            }
                                            if (secondaryExifMetrics.isNotEmpty()) {
                                                Row(
                                                    modifier = Modifier.fillMaxWidth(),
                                                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                                                ) {
                                                    secondaryExifMetrics.forEach { (label, value) ->
                                                        ExifMetric(
                                                            label = label,
                                                            value = value,
                                                            modifier = Modifier.weight(1f),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                DetailGroupCard.FILE_SPECS -> {
                                    Surface(
                                        shape = shape,
                                        color = MaterialTheme.colorScheme.surfaceContainerHigh,
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clip(shape)
                                            .then(
                                                if (fileName != null) {
                                                    Modifier.clickable {
                                                        copyFilenameToClipboard(
                                                            context = context,
                                                            fileName = fileName,
                                                        )
                                                    }
                                                } else {
                                                    Modifier
                                                },
                                            ),
                                    ) {
                                        Row(
                                            modifier = Modifier.padding(16.dp),
                                            verticalAlignment = Alignment.Top,
                                        ) {
                                            Icon(
                                                imageVector = Icons.Outlined.Image,
                                                contentDescription = null,
                                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                                modifier = Modifier
                                                    .padding(top = 2.dp)
                                                    .size(22.dp),
                                            )
                                            Spacer(modifier = Modifier.width(14.dp))
                                            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                                fileName?.let { name ->
                                                    Text(
                                                        text = name,
                                                        style = MaterialTheme.typography.titleSmall.copy(
                                                            fontWeight = FontWeight.Medium,
                                                        ),
                                                        color = MaterialTheme.colorScheme.onSurface,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                    )
                                                }
                                                if (specChips.isNotEmpty()) {
                                                    Text(
                                                        text = specChips.joinToString(" · "),
                                                        style = MaterialTheme.typography.bodySmall,
                                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if (weather != null || windSummary != null) {
                        Spacer(modifier = Modifier.height(2.dp))
                        InfoSummaryRowCard(
                            icon = if (details?.weather?.isDaytime == false) {
                                Icons.Outlined.Nightlight
                            } else {
                                Icons.Outlined.WbSunny
                            },
                            title = weather ?: windSummary.orEmpty(),
                            subtitle = if (weather != null) windSummary else null,
                        )
                    }
                }
            }
        }
    }
}

private enum class DetailGroupCard {
    CAMERA_HEADER,
    EXIF_METRICS,
    FILE_SPECS,
}

private fun groupedCardShape(index: Int, count: Int): RoundedCornerShape {
    val top = if (index == 0) 20.dp else 6.dp
    val bottom = if (index == count - 1) 20.dp else 6.dp
    return RoundedCornerShape(
        topStart = top,
        topEnd = top,
        bottomStart = bottom,
        bottomEnd = bottom,
    )
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.SemiBold),
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun AlbumPreviewRow(
    title: String,
    subtitle: String,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(52.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(MaterialTheme.colorScheme.surfaceContainerHighest),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = Icons.Outlined.PhotoLibrary,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.size(22.dp),
            )
        }
        Spacer(modifier = Modifier.width(14.dp))
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge.copy(fontWeight = FontWeight.Medium),
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun SyncStatusCard(
    syncStatus: SyncStatus,
    isOfflineDetailsMissing: Boolean,
) {
    val (icon, title, subtitle) = when (syncStatus) {
        SyncStatus.SYNCED -> Triple(
            Icons.Outlined.CloudDone,
            "Backed up",
            "Synced between this device and your Ruurd Photos server.",
        )

        SyncStatus.CLOUD_ONLY -> Triple(
            Icons.Outlined.CloudQueue,
            "Backed up · Cloud only",
            "Stored safely on your server. Original file is not on this device.",
        )

        SyncStatus.LOCAL_ONLY -> Triple(
            Icons.Outlined.CloudOff,
            "On device only",
            "Not backed up yet. Folder backup settings control automatic upload.",
        )
    }

    val effectiveSubtitle = if (isOfflineDetailsMissing) {
        "$subtitle Offline — showing cached local and database details."
    } else {
        subtitle
    }

    InfoSummaryRowCard(
        icon = icon,
        title = title,
        subtitle = effectiveSubtitle,
    )
}

@Composable
private fun InfoSummaryRowCard(
    icon: ImageVector,
    title: String,
    subtitle: String? = null,
) {
    Surface(
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = if (subtitle != null) Alignment.Top else Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .then(if (subtitle != null) Modifier.padding(top = 2.dp) else Modifier)
                    .size(22.dp),
            )
            Spacer(modifier = Modifier.width(14.dp))
            Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.SemiBold),
                    color = MaterialTheme.colorScheme.onSurface,
                )
                subtitle?.let { text ->
                    Text(
                        text = text,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun ExifMetric(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Medium),
            color = MaterialTheme.colorScheme.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

private fun copyFilenameToClipboard(context: Context, fileName: String) {
    val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return
    clipboard.setPrimaryClip(ClipData.newPlainText("Filename", fileName))
    Toast.makeText(context, "Filename copied", Toast.LENGTH_SHORT).show()
}

/** `geo:` first; devices without a maps app get the OpenStreetMap web page. */
private fun openMaps(context: Context, latitude: Double, longitude: Double) {
    val geo = Intent(Intent.ACTION_VIEW, MediaInfoFormat.geoUri(latitude, longitude).toUri())
    try {
        context.startActivity(geo)
    } catch (_: ActivityNotFoundException) {
        val web = Intent(
            Intent.ACTION_VIEW,
            MediaInfoFormat.webMapUrl(latitude, longitude).toUri(),
        )
        try {
            context.startActivity(web)
        } catch (_: ActivityNotFoundException) {
            // No browser or maps app; nothing sensible left to open.
        }
    }
}