package dev.ruurd.photos.core.network.api

import dev.ruurd.photos.core.network.dto.LoginRequest
import dev.ruurd.photos.core.network.proto.TimelineSyncResponse
import okhttp3.RequestBody
import okhttp3.ResponseBody
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.Headers
import retrofit2.http.POST
import retrofit2.http.Query

interface PhotosApiService {

    @GET("api/health")
    suspend fun healthCheck(): Response<String>

    @Headers("Content-Type: application/json")
    @POST("api/auth/login")
    suspend fun login(
        @Body request: RequestBody
    ): Response<ResponseBody>

    @POST("api/auth/refresh")
    suspend fun refresh(): Response<ResponseBody>

    @POST("api/auth/logout")
    suspend fun logout(): Response<ResponseBody>

    @GET("api/auth/me")
    suspend fun getMe(): Response<ResponseBody>

    @GET("api/timeline/sync")
    suspend fun getTimelineSync(
        @Query("since") since: Long? = null
    ): Response<TimelineSyncResponse>
}
