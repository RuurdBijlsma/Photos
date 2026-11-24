package dev.ruurd.photos.data.datasource.local

import android.database.Cursor

// Column id only: `MediaStore.MediaColumns.ORIENTATION` is deprecated and may be removed.
internal const val COLUMN_ORIENTATION = "orientation"

internal fun Cursor.getLongOrNull(column: String): Long? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getLong)

internal fun Cursor.getIntOrNull(column: String): Int? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getInt)

internal fun Cursor.getStringOrNull(column: String): String? = getColumnIndex(column).takeIf { it >= 0 && !isNull(it) }?.let(::getString)
