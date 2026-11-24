package dev.ruurd.photos.domain.model

import java.time.Instant
import java.time.LocalDateTime
import java.time.ZoneOffset

/**
 * The server stores capture time as a UTC instant plus the offset that was local at capture, so the
 * wall clock the user saw is that instant shifted by the offset.
 */
fun wallClockEpochMillis(sortTimestamp: Long, timezoneOffsetMinutes: Int): Long = sortTimestamp + timezoneOffsetMinutes * 60_000L

fun wallClockLocalDateTime(sortTimestamp: Long, timezoneOffsetMinutes: Int): LocalDateTime = LocalDateTime.ofInstant(
    Instant.ofEpochMilli(wallClockEpochMillis(sortTimestamp, timezoneOffsetMinutes)),
    ZoneOffset.UTC,
)
