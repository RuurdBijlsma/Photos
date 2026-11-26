package dev.ruurd.photos.core.network.dto

import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * `PUT /api/photos/{cloudId}/item` mirrors the server's `UpdateField<T>`: a field that is not
 * edited must be absent (never `null`, which the server reads as "set to null"), while an explicit
 * clear must serialize as `null`.
 */
class PatchFieldTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
        explicitNulls = true
    }

    @Test
    fun `untouched fields are omitted`() {
        assertEquals("{}", json.encodeToString(UpdateMediaItemRequestDto.serializer(), UpdateMediaItemRequestDto()))
    }

    @Test
    fun `set-null fields serialize as explicit nulls`() {
        assertEquals(
            """{"userCaption":null}""",
            json.encodeToString(
                UpdateMediaItemRequestDto.serializer(),
                UpdateMediaItemRequestDto(userCaption = PatchField.SetNull),
            ),
        )
    }

    @Test
    fun `values serialize next to the fields that changed`() {
        assertEquals(
            """{"userCaption":"caption","orientation":6}""",
            json.encodeToString(
                UpdateMediaItemRequestDto.serializer(),
                UpdateMediaItemRequestDto(
                    userCaption = PatchField.Value("caption"),
                    orientation = 6,
                ),
            ),
        )
    }

    @Test
    fun `timezone and gps keep their own presence`() {
        assertEquals(
            """{"timezoneOffsetSeconds":3600,"gps":null}""",
            json.encodeToString(
                UpdateMediaItemRequestDto.serializer(),
                UpdateMediaItemRequestDto(
                    timezoneOffsetSeconds = PatchField.Value(3600),
                    gps = PatchField.SetNull,
                ),
            ),
        )
    }

    @Test
    fun `ignoring a field explicitly still omits it`() {
        assertEquals(
            """{"orientation":3}""",
            json.encodeToString(
                UpdateMediaItemRequestDto.serializer(),
                UpdateMediaItemRequestDto(
                    userCaption = PatchField.Ignore,
                    timezoneOffsetSeconds = PatchField.Ignore,
                    gps = PatchField.Ignore,
                    orientation = 3,
                ),
            ),
        )
    }
}
