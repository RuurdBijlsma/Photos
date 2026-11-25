package dev.ruurd.photos.features.viewer.components

import androidx.compose.ui.geometry.Rect
import org.junit.Assert.assertEquals
import org.junit.Test

class ViewerGeometryTest {

    @Test
    fun `a wide photo fills the width and is centered vertically`() {
        assertEquals(
            Rect(0f, 375f, 1000f, 625f),
            computeFittedRect(containerWidth = 1000f, containerHeight = 1000f, ratio = 4f),
        )
    }

    @Test
    fun `a tall photo fills the height and is centered horizontally`() {
        assertEquals(
            Rect(375f, 0f, 625f, 1000f),
            computeFittedRect(containerWidth = 1000f, containerHeight = 1000f, ratio = 0.25f),
        )
    }

    @Test
    fun `a degenerate ratio or container falls back instead of dividing by zero`() {
        assertEquals(Rect.Zero, computeFittedRect(0f, 1000f, 1f))
        assertEquals(
            Rect(0f, 0f, 1000f, 1000f),
            computeFittedRect(1000f, 1000f, ratio = Float.NaN),
        )
    }
}
