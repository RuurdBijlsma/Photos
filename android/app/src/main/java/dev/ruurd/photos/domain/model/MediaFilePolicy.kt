package dev.ruurd.photos.domain.model

import java.util.Locale

/** Filenames and extensions the share/copy staging needs when `/item` is not loaded yet. */
object MediaFileNames {

    private val MIME_EXTENSIONS = mapOf(
        "image/jpeg" to "jpg",
        "image/png" to "png",
        "image/webp" to "webp",
        "image/gif" to "gif",
        "image/avif" to "avif",
        "image/heic" to "heic",
        "image/heif" to "heif",
        "image/tiff" to "tiff",
        "image/x-tga" to "tga",
        "video/mp4" to "mp4",
        "video/quicktime" to "mov",
        "video/webm" to "webm",
        "video/x-matroska" to "mkv",
    )

    fun extensionFor(mimeType: String?): String? = MIME_EXTENSIONS[mimeType?.lowercase(Locale.US)]

    /**
     * `"photo_$cloudId.jpg"` / `"video_$cloudId.mp4"`, or the extension the MIME type implies.
     */
    fun fallbackName(cloudId: String, isVideo: Boolean, mimeType: String?): String {
        val extension = extensionFor(mimeType) ?: if (isVideo) "mp4" else "jpg"
        val prefix = if (isVideo) "video" else "photo"
        return "${prefix}_$cloudId.$extension"
    }

    /** Strips path separators so a server-supplied name can never escape the cache directory. */
    fun sanitize(fileName: String, isVideo: Boolean, mimeType: String?): String {
        val cleaned = fileName.substringAfterLast('/').substringAfterLast('\\')
            .replace(Regex("[^A-Za-z0-9._-]"), "_")
            .trim('.', '_')
        if (cleaned.isEmpty()) return fallbackName("media", isVideo, mimeType)
        return if (cleaned.contains('.') && !cleaned.startsWith('.')) cleaned else "$cleaned.${extensionFor(mimeType) ?: "bin"}"
    }
}

/**
 * 2B downloads `/api/photos/{cloudId}/download` into `full_res_cache` for cloud-only stills so
 * Telephoto can subsample them. Originals that Coil cannot subsample (TIFF/TGA) and items whose
 * source file is missing on the server are skipped; videos are streamed by the player in 2C.
 */
object FullResEligibility {

    private val DECODABLE_MIME_TYPES = setOf(
        "image/jpeg",
        "image/jpg",
        "image/png",
        "image/webp",
        "image/gif",
        "image/avif",
        "image/heic",
        "image/heif",
    )

    fun shouldFetchFullRes(
        isVideo: Boolean,
        mimeType: String?,
        missingSince: String?,
    ): Boolean {
        if (isVideo) return false
        if (missingSince != null) return false
        return mimeType?.lowercase(Locale.US) in DECODABLE_MIME_TYPES
    }
}
