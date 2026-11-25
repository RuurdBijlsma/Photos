package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.model.LocalMediaInfo
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.model.mediaInfoUiModel
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
    isOnline: Boolean = true,
    isDetailsLoading: Boolean = false,
    hasDetailsFailed: Boolean = false,
) {
    val model = remember(details, localInfo, item, isOnline, isDetailsLoading, hasDetailsFailed) {
        mediaInfoUiModel(
            details = details,
            localInfo = localInfo,
            item = item,
            isOnline = isOnline,
            isDetailsLoading = isDetailsLoading,
            hasDetailsFailed = hasDetailsFailed,
        )
    }

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
            if (model.captureDateTime != null || model.timezone != null) {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    model.captureDateTime?.let { dateTimeLabel ->
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
                    model.timezone?.let { tz ->
                        Text(
                            text = tz,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // User Caption
            model.caption?.let { caption ->
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
            if (model.albums.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    MediaInfoSectionTitle("Albums")
                    model.albums.forEach { album ->
                        MediaAlbumPreviewRow(
                            title = album.name,
                            subtitle = "In album",
                        )
                    }
                }
            }

            // Location Section
            if ((model.latitude != null && model.longitude != null) || model.placeLabel != null) {
                MediaLocationSection(
                    latitude = model.latitude,
                    longitude = model.longitude,
                    placeLabel = model.placeLabel,
                    coordinates = model.coordinates,
                )
            }

            // Details Section
            if (model.hasDetailsSection) {
                MediaDetailsSection(model = model)
            }
        }
    }
}