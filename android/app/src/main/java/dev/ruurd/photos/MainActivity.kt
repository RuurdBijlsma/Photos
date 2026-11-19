package dev.ruurd.photos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import dev.ruurd.photos.features.main.PhotosAppRoot
import dev.ruurd.photos.ui.theme.RuurdPhotosTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val container = (application as PhotosApplication).container
        setContent {
            RuurdPhotosTheme {
                PhotosAppRoot(container)
            }
        }
    }
}
