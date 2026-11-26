package dev.ruurd.photos.core.permissions

import android.content.Context
import android.net.Uri
import android.provider.MediaStore
import androidx.activity.result.IntentSenderRequest
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * `MediaStore.createWriteRequest` for a photo the app wants to rewrite in place. The viewer observes
 * [requests] with an `IntentSenderRequest` launcher and reports the outcome through [onResult].
 */
class MediaWriteConsent(private val context: Context) {

    private val _requests = MutableStateFlow<IntentSenderRequest?>(null)
    val requests: StateFlow<IntentSenderRequest?> = _requests.asStateFlow()

    private var pending: CompletableDeferred<Boolean>? = null

    /** Shows the system confirmation for [uri]. A newer request supersedes an older one as denied. */
    suspend fun request(uri: Uri): Boolean {
        pending?.complete(false)
        val deferred = CompletableDeferred<Boolean>()
        pending = deferred
        _requests.value = IntentSenderRequest.Builder(
            MediaStore.createWriteRequest(context.contentResolver, listOf(uri)).intentSender,
        ).build()
        return deferred.await()
    }

    fun onResult(granted: Boolean) {
        _requests.value = null
        pending?.complete(granted)
        pending = null
    }
}
