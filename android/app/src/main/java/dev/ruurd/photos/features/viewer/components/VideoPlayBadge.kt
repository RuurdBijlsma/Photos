package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import dev.ruurd.photos.ui.theme.PhotosRadii

/** Centered play badge for a video page, shown until a real player replaces the still frame. */
@Composable
fun VideoPlayBadge(
    durationMs: Int?,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .background(
                color = Color.Black.copy(alpha = 0.6f),
                shape = RoundedCornerShape(PhotosRadii.Card),
            )
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Default.PlayArrow,
            contentDescription = "Video",
            tint = Color.White,
            modifier = Modifier.size(36.dp),
        )
        if (durationMs != null && durationMs > 0) {
            Spacer(modifier = Modifier.width(6.dp))
            val totalSec = durationMs / 1000
            Text(
                text = "%d:%02d".format(totalSec / 60, totalSec % 60),
                style = MaterialTheme.typography.titleMedium,
                color = Color.White,
            )
        }
    }
}
