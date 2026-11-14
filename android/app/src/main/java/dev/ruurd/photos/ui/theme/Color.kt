package dev.ruurd.photos.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

val Purple80 = Color(0xFFD0BCFF)
val PurpleGrey80 = Color(0xFFCCC2DC)
val Pink80 = Color(0xFFEFB8C8)

val Purple40 = Color(0xFF6650a4)
val PurpleGrey40 = Color(0xFF625b71)
val Pink40 = Color(0xFF7D5260)

// Semantic status colors across Light and Dark
val SemanticOfflineLight = Color(0xFFD97706)
val SemanticOfflineDark = Color(0xFFFBBF24)

val SemanticSyncingLight = Color(0xFF2563EB)
val SemanticSyncingDark = Color(0xFF60A5FA)

val SemanticLocalOnlyLight = Color(0xFF6B7280)
val SemanticLocalOnlyDark = Color(0xFF9CA3AF)

val SemanticErrorLight = Color(0xFFDC2626)
val SemanticErrorDark = Color(0xFFF87171)

@Immutable
data class SemanticColors(
    val offline: Color,
    val syncing: Color,
    val localOnly: Color,
    val error: Color
)

val LocalSemanticColors = staticCompositionLocalOf {
    SemanticColors(
        offline = SemanticOfflineLight,
        syncing = SemanticSyncingLight,
        localOnly = SemanticLocalOnlyLight,
        error = SemanticErrorLight
    )
}
