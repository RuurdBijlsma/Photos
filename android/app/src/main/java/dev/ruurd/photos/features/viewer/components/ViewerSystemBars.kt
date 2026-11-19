package dev.ruurd.photos.features.viewer.components

import android.view.Window
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Status/navigation bar appearance for the viewer. Kept out of the screen so the navigation bar
 * stays transparent edge to edge for predictive back, and so the timeline's light appearance is
 * restored both on dispose and when a close animation starts.
 */
@Stable
class ViewerSystemBars(private val window: Window?, private val isDarkTheme: Boolean) {

    private val controller = window?.let { WindowCompat.getInsetsController(it, it.decorView) }

    fun onViewerShown() {
        controller?.apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }
    }

    fun onChromeVisibilityChanged(visible: Boolean) {
        if (visible) {
            controller?.show(WindowInsetsCompat.Type.statusBars())
        } else {
            controller?.hide(WindowInsetsCompat.Type.statusBars())
        }
    }

    fun restoreForTimeline() {
        if (!isDarkTheme) {
            controller?.isAppearanceLightStatusBars = true
            controller?.isAppearanceLightNavigationBars = true
        }
        controller?.show(WindowInsetsCompat.Type.statusBars())
    }
}

@Composable
fun rememberViewerSystemBars(): ViewerSystemBars {
    val window = LocalActivity.current?.window
    val isDarkTheme = isSystemInDarkTheme()
    return remember(window, isDarkTheme) { ViewerSystemBars(window, isDarkTheme) }
}

@Composable
fun ViewerSystemBarsEffect(
    systemBars: ViewerSystemBars,
    isChromeVisible: Boolean,
    isClosing: Boolean,
    onHiddenTimelineMediaChanged: (String?) -> Unit
) {
    val currentOnHiddenChanged = rememberUpdatedState(onHiddenTimelineMediaChanged)

    DisposableEffect(systemBars) {
        systemBars.onViewerShown()
        onDispose {
            currentOnHiddenChanged.value(null)
            systemBars.restoreForTimeline()
        }
    }

    LaunchedEffect(isChromeVisible, isClosing) {
        systemBars.onChromeVisibilityChanged(isChromeVisible || isClosing)
    }
}
