package dev.ruurd.photos.core.network.interceptor

import dev.ruurd.photos.core.network.SessionManager
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.Interceptor
import okhttp3.Response

class DynamicBaseUrlInterceptor(
    private val sessionManager: SessionManager,
) : Interceptor {

    override fun intercept(chain: Interceptor.Chain): Response {
        val originalRequest = chain.request()
        val configuredBaseUrl = sessionManager.getServerUrl().toHttpUrlOrNull()
            ?: return chain.proceed(originalRequest)

        val baseSegments = configuredBaseUrl.encodedPathSegments.filter { it.isNotEmpty() }
        val originalSegments = originalRequest.url.encodedPathSegments
        val needsPrefix = baseSegments.isNotEmpty() && !startsWith(originalSegments, baseSegments)

        val builder = originalRequest.url.newBuilder()
            .scheme(configuredBaseUrl.scheme)
            .host(configuredBaseUrl.host)
            .port(configuredBaseUrl.port)

        if (needsPrefix) {
            val joinedPath = (baseSegments + originalSegments.filter { it.isNotEmpty() })
                .joinToString(separator = "/", prefix = "/")
            builder.encodedPath(joinedPath)
        }

        val newRequest = originalRequest.newBuilder()
            .url(builder.build())
            .build()

        return chain.proceed(newRequest)
    }

    private fun startsWith(segments: List<String>, prefix: List<String>): Boolean {
        if (segments.size < prefix.size) return false
        for (i in prefix.indices) {
            if (segments[i] != prefix[i]) return false
        }
        return true
    }
}
