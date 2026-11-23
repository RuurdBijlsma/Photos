package dev.ruurd.photos.features.viewer.components

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Nightlight
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.MediaInfoFormat
import dev.ruurd.photos.ui.theme.PhotosRadii

/**
 * Swipe-up / About sheet: server `/item` data when available, `MediaStore`/EXIF (or Room, for
 * cloud-only items) otherwise, with a subtle notice when server-only sections are missing.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun MediaInfoBottomSheet(
    details: MediaDetails?,
    localInfo: LocalMediaInfo?,
    item: TimelineMediaProjection?,
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

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
    val metaLine = listOfNotNull(
        MediaInfoFormat.formatResolution(
            details?.width ?: localInfo?.width,
            details?.height ?: localInfo?.height,
        ),
        MediaInfoFormat.formatSize(details?.sizeBytes ?: localInfo?.sizeBytes),
        MediaInfoFormat.formatDuration(details?.durationMs ?: localInfo?.durationMs),
    ).joinToString(" · ")
    val camera = details?.camera ?: localInfo?.camera
    val cameraName = MediaInfoFormat.cameraName(camera)
    val lensName = MediaInfoFormat.lensName(camera)
    val exposurePills = listOfNotNull(
        MediaInfoFormat.exposureLabel(camera?.exposureTime),
        MediaInfoFormat.apertureLabel(camera?.aperture),
        MediaInfoFormat.isoLabel(camera?.iso),
        MediaInfoFormat.focalLabel(camera?.focalLength, camera?.focalLengthIn35mm),
    )
    val weather = MediaInfoFormat.weatherSummary(details?.weather)
    val place = details?.place
    val latitude = place?.latitude ?: localInfo?.latitude
    val longitude = place?.longitude ?: localInfo?.longitude
    val placeLabel = MediaInfoFormat.locationLabel(place)
    val coordinates = MediaInfoFormat.coordinates(latitude, longitude)

    ModalBottomSheet(
        onDismissRequest = onDismissRequest,
        sheetState = sheetState,
        shape = RoundedCornerShape(topStart = PhotosRadii.Sheet, topEnd = PhotosRadii.Sheet),
        containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
        modifier = modifier,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp)
                .padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (fileName != null) {
                Text(
                    text = fileName,
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                    color = MaterialTheme.colorScheme.onSurface,
                )
            }

            if (details == null) {
                Text(
                    text = "Offline — showing local details only.",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            if (captureDateTime != null) {
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        text = captureDateTime,
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    if (timezone != null) {
                        Text(
                            text = timezone,
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            if (metaLine.isNotEmpty()) {
                Text(
                    text = metaLine,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            details?.userCaption?.let { caption ->
                Text(
                    text = caption,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                )
            }

            if (weather != null) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = if (details?.weather?.isDaytime == false) {
                            Icons.Outlined.Nightlight
                        } else {
                            Icons.Outlined.WbSunny
                        },
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp),
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = weather,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            if (cameraName != null || lensName != null || exposurePills.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    cameraName?.let {
                        Text(
                            text = it,
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                    lensName?.let {
                        Text(
                            text = it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (exposurePills.isNotEmpty()) {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            exposurePills.forEach { InfoPill(it) }
                        }
                    }
                }
            }

            if (details?.albums?.isNotEmpty() == true) {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionTitle("Albums")
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        details.albums.forEach { album -> InfoPill(album.name) }
                    }
                }
            }

            if (latitude != null && longitude != null) {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    SectionTitle("Location")
                    placeLabel?.let {
                        Text(
                            text = it,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                    if (coordinates != null) {
                        Text(
                            text = coordinates,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    TextButton(
                        onClick = { openMaps(context, latitude, longitude) },
                        contentPadding = PaddingValues(horizontal = 0.dp, vertical = 4.dp),
                    ) {
                        Text("Open in Maps")
                    }
                }
            }

            Spacer(modifier = Modifier.height(4.dp))
        }
    }
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun InfoPill(text: String) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHighest,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
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
