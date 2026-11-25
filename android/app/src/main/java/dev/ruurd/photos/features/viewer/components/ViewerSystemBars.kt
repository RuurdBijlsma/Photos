package dev.ruurd.photos.features.viewer.components

import android.content.pm.ActivityInfo
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
 * Status/navigation bar appearance and HDR color mode for the viewer. Kept out of the screen so
 * the navigation bar stays transparent edge to edge for predictive back, adapts contrast when the
 * info panel slides over system bars in light theme, and restores the timeline's light appearance
 * and default color mode on close/dispose.
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

    fun onInfoPanelAppearanceChanged(
        isInfoVisible: Boolean,
        isInfoCoveringStatusBar: Boolean,
    ) {
        controller?.apply {
            isAppearanceLightStatusBars = !isDarkTheme && isInfoCoveringStatusBar
            isAppearanceLightNavigationBars = !isDarkTheme && isInfoVisible
        }
    }

    fun setHdrColorMode(enabled: Boolean) {
        val target = if (enabled) ActivityInfo.COLOR_MODE_HDR else ActivityInfo.COLOR_MODE_DEFAULT
        if (window != null && window.colorMode != target) {
            window.colorMode = target
        }
    }

    fun restoreForTimeline() {
        if (!isDarkTheme) {
            controller?.isAppearanceLightStatusBars = true
            controller?.isAppearanceLightNavigationBars = true
        }
        controller?.show(WindowInsetsCompat.Type.statusBars())
        setHdrColorMode(false)
    }
}

/**
 * Coordinates `window.colorMode` across pager transitions so disposing an outgoing page's
 * `SubSamplingImage` does not briefly drop HDR before the incoming page upgrades to full-res.
 */
@Stable
class ViewerHdrCoordinator(
    private val systemBars: ViewerSystemBars,
) {
    private val visiblePreviewHdrPages = HashSet<String>()
    private val visibleSubSamplingPages = HashSet<String>()
    var isViewerActive: Boolean = false

    fun updatePage(
        mediaId: String,
        isVisible: Boolean,
        previewHasGainmap: Boolean,
        hasSubSamplingLayer: Boolean,
    ) {
        if (isVisible && previewHasGainmap) {
            visiblePreviewHdrPages.add(mediaId)
        } else {
            visiblePreviewHdrPages.remove(mediaId)
        }
        if (isVisible && hasSubSamplingLayer) {
            visibleSubSamplingPages.add(mediaId)
        } else {
            visibleSubSamplingPages.remove(mediaId)
        }
    }

    fun removePage(mediaId: String) {
        visiblePreviewHdrPages.remove(mediaId)
        visibleSubSamplingPages.remove(mediaId)
    }

    fun syncWindowColorMode() {
        if (!isViewerActive) return
        when {
            visiblePreviewHdrPages.isNotEmpty() -> systemBars.setHdrColorMode(true)
            visibleSubSamplingPages.isEmpty() -> systemBars.setHdrColorMode(false)

            // When a visible page has a SubSamplingImage mounted and no visible preview carries a
            // gainmap (e.g., cloud-only HDR JPEG whose AVIF preview is SDR), Telephoto owns HDR.
        }
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
    isInfoVisible: Boolean,
    isInfoCoveringStatusBar: Boolean,
    onHiddenTimelineMediaChanged: (String?) -> Unit,
) {
    val currentOnHiddenChanged = rememberUpdatedState(onHiddenTimelineMediaChanged)

    DisposableEffect(systemBars) {
        systemBars.onViewerShown()
        onDispose {
            currentOnHiddenChanged.value(null)
            systemBars.restoreForTimeline()
        }
    }

    LaunchedEffect(isChromeVisible, isClosing, isInfoVisible) {
        systemBars.onChromeVisibilityChanged(isChromeVisible || isClosing || isInfoVisible)
    }

    LaunchedEffect(isClosing, isInfoVisible, isInfoCoveringStatusBar) {
        if (!isClosing) {
            systemBars.onInfoPanelAppearanceChanged(
                isInfoVisible = isInfoVisible,
                isInfoCoveringStatusBar = isInfoCoveringStatusBar,
            )
        }
    }
}
