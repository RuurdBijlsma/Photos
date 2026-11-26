package dev.ruurd.photos.core.network.dto

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder

/**
 * Kotlin counterpart of the server's `UpdateField<T>` (`untagged`: value, `null`, or absent).
 *
 * [Ignore] must stay the DTO default and the Json instance must use `encodeDefaults = false`, so an
 * untouched field is left out of the body entirely. Encoding a `null` for it would make the server
 * clear that column.
 */
@Serializable(with = PatchFieldSerializer::class)
sealed class PatchField<out T> {
    data object Ignore : PatchField<Nothing>()

    data object SetNull : PatchField<Nothing>()

    data class Value<T>(val value: T) : PatchField<T>()
}

class PatchFieldSerializer<T>(
    private val valueSerializer: KSerializer<T>,
) : KSerializer<PatchField<T>> {
    override val descriptor: SerialDescriptor = valueSerializer.descriptor

    @OptIn(ExperimentalSerializationApi::class)
    override fun serialize(encoder: Encoder, value: PatchField<T>) {
        when (value) {
            PatchField.Ignore -> throw SerializationException(
                "PatchField.Ignore is omitted by the DTO default; use encodeDefaults = false",
            )

            PatchField.SetNull -> encoder.encodeNull()

            is PatchField.Value -> encoder.encodeSerializableValue(valueSerializer, value.value)
        }
    }

    override fun deserialize(decoder: Decoder): PatchField<T> = PatchField.Value(decoder.decodeSerializableValue(valueSerializer))
}
