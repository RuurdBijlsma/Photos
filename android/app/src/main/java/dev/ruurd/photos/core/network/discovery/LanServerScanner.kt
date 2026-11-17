package dev.ruurd.photos.core.network.discovery

import android.os.Build
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.core.network.await
import dev.ruurd.photos.domain.model.DiscoveredServer
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import okhttp3.Dispatcher
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import java.net.Inet4Address
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

sealed interface LanScanEvent {
    data class Found(val server: DiscoveredServer) : LanScanEvent
    data class Progress(val completed: Int, val total: Int) : LanScanEvent
    data object NoLocalNetwork : LanScanEvent
}

/**
 * Probes the local Wi-Fi / Ethernet `/24` subnet (plus `10.0.2.2` on emulators) for Ruurd Photos
 * servers using `/api/health`. Works across both native host runs (`5272`) and Docker bridge
 * deployments (`9475`, or `80`/`443` via reverse proxy).
 */
class LanServerScanner(
    private val networkMonitor: NetworkMonitor,
    directClient: OkHttpClient,
    private val dispatchers: AppDispatchers
) {
    private val probeClient: OkHttpClient = directClient.newBuilder()
        .dispatcher(
            Dispatcher().apply {
                maxRequests = MAX_CONCURRENCY + 8
                maxRequestsPerHost = 8
            }
        )
        .connectTimeout(CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        .readTimeout(READ_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        .writeTimeout(READ_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        .retryOnConnectionFailure(false)
        .followRedirects(false)
        .build()

    fun scan(prioritizedHost: String? = null): Flow<LanScanEvent> = channelFlow {
        val localIp = networkMonitor.lanIpv4Address.value
        val emulator = isEmulator()

        if (localIp == null && !emulator) {
            send(LanScanEvent.NoLocalNetwork)
            return@channelFlow
        }

        val primaryTargets = buildTargets(localIp, emulator, PRIMARY_PORTS, prioritizedHost)
        val proxyTargets = buildTargets(localIp, emulator = false, PROXY_PORTS, prioritizedHost)
        val total = primaryTargets.size + proxyTargets.size
        if (total == 0) {
            send(LanScanEvent.NoLocalNetwork)
            return@channelFlow
        }

        val completed = AtomicInteger(0)
        val semaphore = Semaphore(MAX_CONCURRENCY)

        // Probe dedicated Ruurd Photos ports (5272, 9475) first so matches appear quickly before
        // checking generic HTTP/HTTPS reverse-proxy ports (80, 443).
        probeBatch(
            targets = primaryTargets,
            semaphore = semaphore,
            completed = completed,
            total = total,
            onFound = { send(LanScanEvent.Found(it)) },
            onProgress = { done -> send(LanScanEvent.Progress(done, total)) }
        )

        probeBatch(
            targets = proxyTargets,
            semaphore = semaphore,
            completed = completed,
            total = total,
            onFound = { send(LanScanEvent.Found(it)) },
            onProgress = { done -> send(LanScanEvent.Progress(done, total)) }
        )

        send(LanScanEvent.Progress(total, total))
    }.flowOn(dispatchers.io)

    private suspend fun probeBatch(
        targets: List<ProbeTarget>,
        semaphore: Semaphore,
        completed: AtomicInteger,
        total: Int,
        onFound: suspend (DiscoveredServer) -> Unit,
        onProgress: suspend (Int) -> Unit
    ) {
        if (targets.isEmpty()) return
        coroutineScope {
            for (target in targets) {
                launch {
                    val found = semaphore.withPermit { probe(target) }
                    if (found != null) {
                        onFound(found)
                    }
                    val done = completed.incrementAndGet()
                    if (done % PROGRESS_STEP == 0 || done == total) {
                        onProgress(done)
                    }
                }
            }
        }
    }

    private suspend fun probe(target: ProbeTarget): DiscoveredServer? {
        val url = target.baseUrl
        val request = Request.Builder()
            .url("$url/api/health")
            .get()
            .build()

        val startNanos = System.nanoTime()
        return try {
            probeClient.newCall(request).await().use { response ->
                if (!isRuurdPhotosHealthResponse(response, target.port)) return@use null
                val latencyMs = ((System.nanoTime() - startNanos) / 1_000_000L).coerceAtLeast(1L)
                DiscoveredServer(
                    url = url,
                    host = target.host,
                    port = target.port,
                    latencyMs = latencyMs,
                    isEmulatorHost = target.isEmulatorHost
                )
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }
    }

    private fun isRuurdPhotosHealthResponse(response: Response, port: Int): Boolean {
        if (!response.isSuccessful) return false
        if (response.header(RUURD_PHOTOS_HEADER).equals("1", ignoreCase = true)) return true

        // Fallback for servers that don't yet emit X-Ruurd-Photos: only accept the default
        // Ruurd Photos ports and reject HTML responses (e.g. generic nginx SPA catch-alls).
        if (PRIMARY_PORTS.none { it.port == port }) return false
        val contentType = response.header("Content-Type")?.lowercase().orEmpty()
        if (contentType.contains("text/html")) return false
        val body = response.peekBody(128).string().trim()
        return body.isNotEmpty() && !body.startsWith("<")
    }

    private fun buildTargets(
        localIp: Inet4Address?,
        emulator: Boolean,
        ports: List<PortSpec>,
        prioritizedHost: String?
    ): List<ProbeTarget> {
        val hosts = LinkedHashSet<HostCandidate>()

        if (emulator) {
            hosts += HostCandidate(EMULATOR_HOST, isEmulatorHost = true)
        }

        if (localIp != null) {
            val bytes = localIp.address
            val prefix = "${bytes[0].toInt() and 0xFF}.${bytes[1].toInt() and 0xFF}.${bytes[2].toInt() and 0xFF}"
            val selfOctet = bytes[3].toInt() and 0xFF

            if (!prioritizedHost.isNullOrBlank() &&
                prioritizedHost.startsWith("$prefix.") &&
                prioritizedHost != localIp.hostAddress
            ) {
                hosts += HostCandidate(prioritizedHost, isEmulatorHost = false)
            }

            for (octet in 1..254) {
                if (octet == selfOctet) continue
                hosts += HostCandidate("$prefix.$octet", isEmulatorHost = false)
            }
        }

        val targets = ArrayList<ProbeTarget>(hosts.size * ports.size)
        for (host in hosts) {
            for (port in ports) {
                targets += ProbeTarget(
                    scheme = port.scheme,
                    host = host.host,
                    port = port.port,
                    omitDefaultPort = port.omitDefaultPort,
                    isEmulatorHost = host.isEmulatorHost
                )
            }
        }
        return targets
    }

    private fun isEmulator(): Boolean =
        Build.HARDWARE.contains("ranchu", ignoreCase = true) ||
                Build.HARDWARE.contains("goldfish", ignoreCase = true) ||
                Build.FINGERPRINT.startsWith("generic", ignoreCase = true) ||
                Build.FINGERPRINT.contains("emulator", ignoreCase = true) ||
                Build.MODEL.contains("Emulator", ignoreCase = true) ||
                Build.MODEL.contains("Android SDK built for", ignoreCase = true)

    private data class HostCandidate(val host: String, val isEmulatorHost: Boolean)

    private data class PortSpec(
        val scheme: String,
        val port: Int,
        val omitDefaultPort: Boolean = false
    )

    private data class ProbeTarget(
        val scheme: String,
        val host: String,
        val port: Int,
        val omitDefaultPort: Boolean,
        val isEmulatorHost: Boolean
    ) {
        val baseUrl: String
            get() = if (omitDefaultPort) "$scheme://$host" else "$scheme://$host:$port"
    }

    private companion object {
        const val EMULATOR_HOST = "10.0.2.2"
        const val RUURD_PHOTOS_HEADER = "X-Ruurd-Photos"
        const val MAX_CONCURRENCY = 48
        const val CONNECT_TIMEOUT_MS = 450L
        const val READ_TIMEOUT_MS = 800L
        const val PROGRESS_STEP = 16

        val PRIMARY_PORTS = listOf(
            PortSpec("http", 5272),
            PortSpec("http", 9475)
        )
        val PROXY_PORTS = listOf(
            PortSpec("http", 80, omitDefaultPort = true),
            PortSpec("https", 443, omitDefaultPort = true)
        )
    }
}