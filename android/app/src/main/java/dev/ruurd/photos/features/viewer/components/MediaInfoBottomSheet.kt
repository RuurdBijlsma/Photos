package dev.ruurd.photos.features.viewer.components

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
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
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Face
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Map
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
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

    val captureDateTime = MediaInfoFormat.captureDateTime(details)
        ?: item?.let { projection ->
            MediaInfoFormat.offlineCaptureDateTime(
                projection.sortTimestamp,
                projection.timezoneOffsetMinutes,
            )
        }
        ?: localInfo?.dateTakenMs?.let { MediaInfoFormat.epochDateTime(it) }
    val timezone = MediaInfoFormat.timezoneLabel(details?.timezoneOffsetSeconds, details?.timezoneName)
    val fileName = details?.fileName ?: localInfo?.fileName
    val width = details?.width ?: localInfo?.width
    val height = details?.height ?: localInfo?.height
    val megapixels = MediaInfoFormat.formatMegapixels(width, height)
    val resolution = MediaInfoFormat.formatResolution(width, height)
    val fileSize = MediaInfoFormat.formatSize(details?.sizeBytes ?: localInfo?.sizeBytes)
    val duration = MediaInfoFormat.formatDuration(details?.durationMs ?: localInfo?.durationMs)
    val mimeType = details?.mimeType ?: localInfo?.mimeType ?: if (item?.isVideo == true) "video/*" else "image/*"

    val camera = details?.camera ?: localInfo?.camera
    val cameraName = MediaInfoFormat.cameraName(camera)
    val lensName = MediaInfoFormat.lensName(camera)
    val aperture = MediaInfoFormat.apertureLabel(camera?.aperture)
    val shutter = MediaInfoFormat.exposureLabel(camera?.exposureTime)
    val focal = MediaInfoFormat.focalLabel(camera?.focalLength, camera?.focalLengthIn35mm)
    val iso = MediaInfoFormat.isoLabel(camera?.iso)
    val exposureBias = MediaInfoFormat.exposureCompensationLabel(camera?.exposureCompensation)

    val weather = MediaInfoFormat.weatherSummary(details?.weather)
    val place = details?.place
    val latitude = place?.latitude ?: localInfo?.latitude
    val longitude = place?.longitude ?: localInfo?.longitude
    val placeLabel = MediaInfoFormat.locationLabel(place)
    val coordinates = MediaInfoFormat.coordinates(latitude, longitude)

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
            verticalArrangement = Arrangement.spacedBy(20.dp),
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
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.Top,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text(
                        text = captureDateTime ?: "Date unknown",
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = timezone ?: "Local timezone",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Icon(
                    imageVector = Icons.Outlined.Edit,
                    contentDescription = "Edit date and time",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier
                        .padding(start = 12.dp, top = 2.dp)
                        .size(20.dp),
                )
            }

            // User Caption (or placeholder)
            Surface(
                shape = RoundedCornerShape(PhotosRadii.NestedCard),
                color = MaterialTheme.colorScheme.surfaceContainer,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = details?.userCaption ?: "Add a caption...",
                    style = MaterialTheme.typography.bodyLarge,
                    color = if (details?.userCaption != null) {
                        MaterialTheme.colorScheme.onSurface
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    },
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                )
            }

            // Albums Section (real albums when present, filler rows otherwise for layout testing)
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                SectionTitle("Albums")
                val albums = details?.albums.orEmpty()
                if (albums.isNotEmpty()) {
                    albums.forEach { album ->
                        AlbumPreviewRow(
                            title = album.name,
                            subtitle = "In album",
                        )
                    }
                } else {
                    AlbumPreviewRow(
                        title = "Travel & Highlights",
                        subtitle = "Sample album · 248 items (preview)",
                    )
                    AlbumPreviewRow(
                        title = "Favorites",
                        subtitle = "Sample album · 94 items (preview)",
                    )
                }
            }

            // Location & Map View Section
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(
                        modifier = Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        Text(
                            text = placeLabel ?: if (coordinates != null) "Pinned location" else "No location data",
                            style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.SemiBold),
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = coordinates ?: "Location metadata unavailable",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (latitude != null && longitude != null) {
                        TextButton(
                            onClick = { openMaps(context, latitude, longitude) },
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        ) {
                            Text("Open in Maps")
                        }
                    }
                }

                // Map View Placeholder Card
                Surface(
                    shape = RoundedCornerShape(20.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(156.dp)
                        .clip(RoundedCornerShape(20.dp))
                        .then(
                            if (latitude != null && longitude != null) {
                                Modifier.clickable { openMaps(context, latitude, longitude) }
                            } else {
                                Modifier
                            },
                        ),
                ) {
                    Box(
                        modifier = Modifier.padding(16.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(44.dp)
                                    .clip(CircleShape)
                                    .background(MaterialTheme.colorScheme.primaryContainer),
                                contentAlignment = Alignment.Center,
                            ) {
                                Icon(
                                    imageVector = if (latitude != null && longitude != null) {
                                        Icons.Outlined.LocationOn
                                    } else {
                                        Icons.Outlined.Map
                                    },
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onPrimaryContainer,
                                )
                            }
                            Text(
                                text = if (latitude != null && longitude != null) {
                                    "Map preview coming soon · Tap to open in Maps"
                                } else {
                                    "Map view coming soon"
                                },
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            // Details Section (Cloud Sync Status, Camera & EXIF, File Specs, Weather, People)
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                SectionTitle("Details")

                // Cloud Sync Status Card
                SyncStatusCard(
                    syncStatus = item?.syncStatus,
                    isOfflineDetailsMissing = details == null && item?.cloudId != null,
                )

                Spacer(modifier = Modifier.height(2.dp))

                // Expanded Camera & EXIF Card Group
                Surface(
                    shape = RoundedCornerShape(topStart = 20.dp, topEnd = 20.dp, bottomStart = 6.dp, bottomEnd = 6.dp),
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
                            Text(
                                text = cameraName ?: "Camera metadata unavailable",
                                style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.SemiBold),
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            Text(
                                text = lensName ?: "Standard lens · Orientation ${details?.orientation ?: 1}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }

                Surface(
                    shape = RoundedCornerShape(6.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            ExifMetric(label = "Aperture", value = aperture ?: "—", modifier = Modifier.weight(1f))
                            ExifMetric(label = "Shutter", value = shutter ?: "—", modifier = Modifier.weight(1f))
                            ExifMetric(label = "Focal", value = focal ?: "—", modifier = Modifier.weight(1f))
                            ExifMetric(label = "ISO", value = iso ?: "—", modifier = Modifier.weight(1f))
                        }
                        if (exposureBias != null || camera?.focalLengthIn35mm != null) {
                            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f))
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(16.dp),
                            ) {
                                exposureBias?.let {
                                    ExifMetric(label = "Exposure bias", value = it, modifier = Modifier.weight(1f))
                                }
                                camera?.focalLengthIn35mm?.takeIf { it > 0f }?.let { eq35 ->
                                    ExifMetric(
                                        label = "35mm equiv.",
                                        value = "${eq35.toInt()} mm",
                                        modifier = Modifier.weight(1f),
                                    )
                                }
                            }
                        }
                    }
                }

                // File & Storage Specs Card
                Surface(
                    shape = RoundedCornerShape(topStart = 6.dp, topEnd = 6.dp, bottomStart = 20.dp, bottomEnd = 20.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.Top,
                    ) {
                        Icon(
                            imageVector = Icons.Outlined.Image,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(22.dp),
                        )
                        Spacer(modifier = Modifier.width(14.dp))
                        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(
                                text = fileName ?: item?.id ?: "Unknown file",
                                style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.Medium),
                                color = MaterialTheme.colorScheme.onSurface,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val specChips = listOfNotNull(
                                megapixels,
                                resolution,
                                fileSize,
                                duration,
                                mimeType,
                            )
                            Text(
                                text = specChips.joinToString(" · "),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(2.dp))

                // Weather Card
                InfoSummaryRowCard(
                    icon = if (details?.weather?.isDaytime == false) {
                        Icons.Outlined.Nightlight
                    } else {
                        Icons.Outlined.WbSunny
                    },
                    title = weather ?: "Weather at capture",
                    subtitle = if (weather != null) {
                        details?.weather?.windSpeed?.let { "Wind %.1f km/h".format(it) } ?: "Recorded weather"
                    } else {
                        "Historical weather data unavailable for this item"
                    },
                )

                // People & Tags Filler Card (ensures tall scroll depth for testing)
                InfoSummaryRowCard(
                    icon = Icons.Outlined.Face,
                    title = "People & Visual Tags",
                    subtitle = "Recognized faces, objects, and semantic tags will appear here",
                )
            }
        }
    }
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
    syncStatus: SyncStatus?,
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

        SyncStatus.LOCAL_ONLY, null -> Triple(
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
    subtitle: String,
) {
    Surface(
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.Top,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .padding(top = 2.dp)
                    .size(22.dp),
            )
            Spacer(modifier = Modifier.width(14.dp))
            Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.SemiBold),
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
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