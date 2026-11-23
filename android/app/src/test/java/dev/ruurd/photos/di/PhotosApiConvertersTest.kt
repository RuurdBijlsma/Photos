package dev.ruurd.photos.di

import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import kotlinx.coroutines.runBlocking
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Retrofit

class PhotosApiConvertersTest {
    /**
     * Retrofit resolves the response converter on the first call, so this fails with a
     * SerializationException if kotlinx-serialization is registered before the protobuf one.
     */
    @Test
    fun `sync response decodes as protobuf`() = runBlocking {
        val payload = TimelineSyncResponse.getDefaultInstance().toByteArray()
        val client = OkHttpClient.Builder()
            .addInterceptor { chain ->
                Response.Builder()
                    .request(chain.request())
                    .protocol(Protocol.HTTP_1_1)
                    .code(200)
                    .message("OK")
                    .body(payload.toResponseBody("application/x-protobuf".toMediaType()))
                    .build()
            }
            .build()
        val service = Retrofit.Builder()
            .baseUrl("http://localhost/")
            .client(client)
            .apply { photosApiConverterFactories().forEach { addConverterFactory(it) } }
            .build()
            .create(PhotosApiService::class.java)

        assertTrue(service.getTimelineSync().isSuccessful)
    }
}
