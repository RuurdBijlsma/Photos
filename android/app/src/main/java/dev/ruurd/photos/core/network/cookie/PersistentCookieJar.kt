package dev.ruurd.photos.core.network.cookie

import android.content.Context
import android.content.SharedPreferences
import okhttp3.Cookie
import okhttp3.CookieJar
import okhttp3.HttpUrl
import java.util.concurrent.ConcurrentHashMap

class PersistentCookieJar(context: Context) : CookieJar {

    private val sharedPrefs: SharedPreferences =
        context.getSharedPreferences("cookie_store", Context.MODE_PRIVATE)

    // Key format: "${cookie.domain}|${cookie.path}|${cookie.name}"
    private val memoryStore = ConcurrentHashMap<String, Cookie>()

    init {
        loadPersistedCookies()
    }

    private fun loadPersistedCookies() {
        val now = System.currentTimeMillis()
        val expiredPrefsKeys = mutableListOf<String>()

        for ((prefsKey, serialized) in sharedPrefs.all) {
            if (serialized !is String) continue
            val cookie = deserializeCookie(serialized)
            if (cookie != null && cookie.expiresAt > now) {
                memoryStore[cookieKey(cookie)] = cookie
            } else {
                expiredPrefsKeys.add(prefsKey)
            }
        }

        if (expiredPrefsKeys.isNotEmpty()) {
            val editor = sharedPrefs.edit()
            for (key in expiredPrefsKeys) {
                editor.remove(key)
            }
            editor.apply()
        }
    }

    @Synchronized
    override fun saveFromResponse(url: HttpUrl, cookies: List<Cookie>) {
        val editor = sharedPrefs.edit()
        val now = System.currentTimeMillis()

        for (cookie in cookies) {
            val key = cookieKey(cookie)
            if (cookie.expiresAt <= now) {
                memoryStore.remove(key)
                editor.remove(key)
            } else {
                memoryStore[key] = cookie
                editor.putString(key, serializeCookie(cookie))
            }
        }
        editor.apply()
    }

    @Synchronized
    override fun loadForRequest(url: HttpUrl): List<Cookie> {
        val now = System.currentTimeMillis()
        val matchingCookies = mutableListOf<Cookie>()
        val expiredKeys = mutableListOf<String>()

        for ((key, cookie) in memoryStore) {
            if (cookie.expiresAt <= now) {
                expiredKeys.add(key)
            } else if (cookie.matches(url)) {
                matchingCookies.add(cookie)
            }
        }

        if (expiredKeys.isNotEmpty()) {
            val editor = sharedPrefs.edit()
            for (key in expiredKeys) {
                memoryStore.remove(key)
                editor.remove(key)
            }
            editor.apply()
        }

        return matchingCookies
    }

    @Synchronized
    fun getCookie(name: String, url: HttpUrl): Cookie? {
        val now = System.currentTimeMillis()
        return memoryStore.values.firstOrNull { cookie ->
            cookie.name == name && cookie.expiresAt > now && cookie.matches(url)
        }
    }

    @Synchronized
    fun clear() {
        memoryStore.clear()
        sharedPrefs.edit().clear().apply()
    }

    private fun cookieKey(cookie: Cookie): String =
        "${cookie.domain}|${cookie.path}|${cookie.name}"

    private fun serializeCookie(cookie: Cookie): String {
        val sb = StringBuilder()
        sb.append(cookie.name).append("=").append(cookie.value)
        sb.append(";expires=").append(cookie.expiresAt)
        sb.append(";domain=").append(cookie.domain)
        sb.append(";path=").append(cookie.path)
        if (cookie.secure) sb.append(";secure")
        if (cookie.httpOnly) sb.append(";httponly")
        if (cookie.hostOnly) sb.append(";hostonly")
        return sb.toString()
    }

    private fun deserializeCookie(str: String): Cookie? {
        return try {
            val parts = str.split(";")
            if (parts.isEmpty()) return null

            val nameValue = parts[0].split("=", limit = 2)
            if (nameValue.size != 2) return null

            val builder = Cookie.Builder()
                .name(nameValue[0].trim())
                .value(nameValue[1].trim())

            var domain: String? = null
            var hostOnly = false

            for (i in 1 until parts.size) {
                val part = parts[i].trim()
                when {
                    part.startsWith("expires=") -> {
                        val exp = part.removePrefix("expires=").toLongOrNull() ?: continue
                        builder.expiresAt(exp)
                    }
                    part.startsWith("domain=") -> {
                        domain = part.removePrefix("domain=").trim().takeIf { it.isNotEmpty() }
                    }
                    part.startsWith("path=") -> {
                        val path = part.removePrefix("path=").trim()
                        if (path.isNotEmpty()) builder.path(path)
                    }
                    part.equals("secure", ignoreCase = true) -> builder.secure()
                    part.equals("httponly", ignoreCase = true) -> builder.httpOnly()
                    part.equals("hostonly", ignoreCase = true) -> hostOnly = true
                }
            }

            if (domain != null) {
                if (hostOnly) {
                    builder.hostOnlyDomain(domain)
                } else {
                    builder.domain(domain)
                }
            }

            builder.build()
        } catch (_: Exception) {
            null
        }
    }
}