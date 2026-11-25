package dev.ruurd.photos.core.network.api

import dev.ruurd.photos.core.network.dto.MediaItemWithAlbumsDto
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import okhttp3.RequestBody
import okhttp3.ResponseBody
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.Headers
import retrofit2.http.POST
import retrofit2.http.Path
import retrofit2.http.Query

/**
 * Authenticated Retrofit endpoints. Note that `/api/auth/refresh` and `/api/auth/logout` are
 * called directly via dedicated short-timeout OkHttp clients in `AuthAuthenticator` and
 * `AuthRepositoryImpl` (to avoid recursive 401 loops and offline sign-out hangs).
 */
interface PhotosApiService {

    @GET("api/health")
    suspend fun healthCheck(): Response<String>

    @Headers("Content-Type: application/json")
    @POST("api/auth/login")
    suspend fun login(
        @Body request: RequestBody,
    ): Response<ResponseBody>

    @GET("api/auth/me")
    suspend fun getMe(): Response<ResponseBody>

    @GET("api/timeline/sync")
    suspend fun getTimelineSync(
        @Query("since") since: Long? = null,
    ): Response<TimelineSyncResponse>

    @GET("api/photos/{cloudId}/item")
    suspend fun getMediaItem(
        @Path("cloudId") cloudId: String,
    ): Response<MediaItemWithAlbumsDto>
}
