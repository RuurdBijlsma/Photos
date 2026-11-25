package dev.ruurd.photos.features.viewer.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import dev.ruurd.photos.domain.model.VideoPlaybackPolicy
import dev.ruurd.photos.ui.theme.PhotosRadii
import kotlin.math.abs

@Composable
fun PlaybackSpeedDialog(
    currentSpeed: Float,
    onSelectSpeed: (Float) -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(PhotosRadii.Dialog),
        title = { Text("Playback speed") },
        modifier = modifier,
        text = {
            Column(modifier = Modifier.fillMaxWidth()) {
                VideoPlaybackPolicy.PLAYBACK_SPEEDS.forEach { speed ->
                    val selected = abs(currentSpeed - speed) < 0.01f
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(12.dp))
                            .clickable {
                                onSelectSpeed(speed)
                                onDismiss()
                            }
                            .padding(horizontal = 4.dp, vertical = 6.dp),
                    ) {
                        RadioButton(
                            selected = selected,
                            onClick = null,
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = if (speed == 1.0f) "1x (Normal)" else VideoPlaybackPolicy.formatSpeed(speed),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    }
                }
                Spacer(modifier = Modifier.height(4.dp))
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text("Close")
            }
        },
    )
}
