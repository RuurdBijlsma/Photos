package dev.ruurd.photos

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.ruurd.photos.core.common.DefaultAppDispatchers
import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.TimelineMediaProjection
import dev.ruurd.photos.domain.repository.MediaRepository
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.domain.usecase.ObserveTimelineUseCase
import dev.ruurd.photos.features.timeline.TimelineScreen
import dev.ruurd.photos.features.timeline.TimelineViewModel
import dev.ruurd.photos.ui.theme.RuurdPhotosTheme
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.time.LocalDate
import java.time.ZoneOffset
import java.util.concurrent.Executors

class BenchmarkTimelineActivity : ComponentActivity() {

    @Volatile
    private var thumbServer: LocalThumbnailServer? = null

    @Volatile
    private var isDestroyedFlag = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val app = application as PhotosApplication
        val dispatchers = DefaultAppDispatchers()
        val repo = SeededTimelineMediaRepository(count = 2_000)
        val observeUseCase = ObserveTimelineUseCase(repo, dispatchers)
        val layoutUseCase = CalculateTimelineLayoutUseCase()

        setContent {
            RuurdPhotosTheme {
                var readyBaseUrl by remember { mutableStateOf<String?>(null) }

                LaunchedEffect(Unit) {
                    val url = withContext(dispatchers.io) {
                        val server = LocalThumbnailServer().also {
                            if (isDestroyedFlag) {
                                it.close()
                                return@withContext null
                            }
                            it.start()
                            thumbServer = it
                        }
                        // DynamicBaseUrlInterceptor rewrites requests to SessionManager's URL.
                        app.container.sessionManager.setServerUrl(server.baseUrl)
                        server.baseUrl
                    }
                    if (url != null) {
                        readyBaseUrl = url
                    }
                }

                val baseUrl = readyBaseUrl
                if (baseUrl == null) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(MaterialTheme.colorScheme.background),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                } else {
                    val timelineViewModel: TimelineViewModel = viewModel(
                        factory = TimelineViewModel.factory(
                            observeTimelineUseCase = observeUseCase,
                            calculateTimelineLayoutUseCase = layoutUseCase,
                            dispatchers = dispatchers
                        )
                    )
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(MaterialTheme.colorScheme.background)
                    ) {
                        TimelineScreen(
                            viewModel = timelineViewModel,
                            baseUrl = baseUrl,
                            contentPadding = PaddingValues(top = 56.dp, bottom = 86.dp),
                            topBarHeight = 56.dp,
                            topBarVisible = true,
                            isSyncing = false
                        )
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        isDestroyedFlag = true
        thumbServer?.close()
        thumbServer = null
        super.onDestroy()
    }
}

private class LocalThumbnailServer {
    private val socket = ServerSocket(
        0,
        64,
        InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1))
    )
    private val pool = Executors.newFixedThreadPool(4)
    private val fixtures: List<ByteArray> = List(12) { index -> renderJpegFixture(index) }

    @Volatile
    private var running = true

    val baseUrl: String get() = "http://127.0.0.1:${socket.localPort}"

    fun start() {
        Thread({
            while (running) {
                try {
                    val client = socket.accept()
                    pool.execute { handle(client) }
                } catch (_: Throwable) {
                    if (!running) break
                }
            }
        }, "bench-thumb-server").start()
    }

    private fun handle(client: Socket) {
        try {
            client.use { s ->
                val reader = s.getInputStream().bufferedReader()
                val requestLine = reader.readLine() ?: return
                while (true) {
                    val line = reader.readLine() ?: break
                    if (line.isEmpty()) break
                }
                val key = requestLine.hashCode() and 0x7FFFFFFF
                val body = fixtures[key % fixtures.size]
                val header = buildString {
                    append("HTTP/1.1 200 OK\r\n")
                    append("Content-Type: image/jpeg\r\n")
                    append("Content-Length: ${body.size}\r\n")
                    append("Connection: close\r\n\r\n")
                }.toByteArray()

                val out = s.getOutputStream()
                out.write(header)
                out.write(body)
                out.flush()
            }
        } catch (_: Throwable) {
            // Client closed early during fast scroll or activity teardown; ignore.
        }
    }

    fun close() {
        running = false
        runCatching { socket.close() }
        pool.shutdownNow()
    }

    private fun renderJpegFixture(seed: Int): ByteArray {
        val bmp = Bitmap.createBitmap(480, 360, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        val hue = (seed * 30f) % 360f
        canvas.drawColor(Color.HSVToColor(floatArrayOf(hue, 0.45f, 0.75f)))
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.HSVToColor(floatArrayOf((hue + 140f) % 360f, 0.5f, 0.9f))
        }
        canvas.drawCircle(160f + (seed % 4) * 40f, 180f, 95f, paint)
        return ByteArrayOutputStream().use { out ->
            bmp.compress(Bitmap.CompressFormat.JPEG, 82, out)
            bmp.recycle()
            out.toByteArray()
        }
    }
}

private class SeededTimelineMediaRepository(count: Int) : MediaRepository {
    private val ratios = floatArrayOf(1.333f, 0.75f, 1.777f, 1.0f, 0.667f, 1.5f)
    private val items: List<TimelineMediaProjection> = buildList(count) {
        val startDay = LocalDate.of(2026, 11, 15)
        for (i in 0 until count) {
            val day = startDay.minusDays((i / 3).toLong())
            val ym = day.year * 100 + day.monthValue
            val ts = day.atTime(12, 0).toInstant(ZoneOffset.UTC).toEpochMilli() - (i % 3) * 60_000L
            val isVideo = i % 11 == 0
            add(
                TimelineMediaProjection(
                    id = "bench_$i",
                    cloudId = "c_${i % 12}",
                    localContentUri = null,
                    yearMonth = ym,
                    ratio = ratios[i % ratios.size],
                    isVideo = isVideo,
                    durationMs = if (isVideo) 14_000 else null,
                    hasThumbnails = false,
                    syncStatus = SyncStatus.CLOUD_ONLY,
                    sortTimestamp = ts,
                    timezoneOffsetMinutes = 0
                )
            )
        }
    }

    private val stage2Flow = MutableStateFlow(items)

    override fun getTimelineStage1(limitMonths: Int): Flow<List<TimelineMediaProjection>> {
        val topMonths = items.asSequence().map { it.yearMonth }.distinct().take(limitMonths).toHashSet()
        return flowOf(items.filter { it.yearMonth in topMonths })
    }

    override fun getTimelineStage2(): Flow<List<TimelineMediaProjection>> = stage2Flow
    override suspend fun getById(id: String): MediaEntity? = null
    override suspend fun getByCloudId(cloudId: String): MediaEntity? = null
    override suspend fun getAllByCloudId(cloudId: String): List<MediaEntity> = emptyList()
    override suspend fun getLocalRows(isVideo: Boolean): List<MediaEntity> = emptyList()
    override suspend fun findBySizeAndStatus(sizeBytes: Long, syncStatus: SyncStatus): List<MediaEntity> = emptyList()
    override suspend fun getAllCloudIds(): List<String> = emptyList()
    override suspend fun upsert(media: MediaEntity) = Unit
    override suspend fun upsertAll(mediaList: List<MediaEntity>) = Unit
    override suspend fun deleteById(id: String) = Unit
    override suspend fun transitionToLocalOnlyByCloudIds(cloudIds: List<String>) = Unit
    override suspend fun deleteCloudOnlyByCloudIds(cloudIds: List<String>) = Unit
    override suspend fun deleteByCloudId(cloudId: String) = Unit
    override suspend fun deleteByCloudIds(cloudIds: List<String>) = Unit
    override suspend fun clearAll() = Unit
    override suspend fun <T> withTransaction(block: suspend () -> T): T = block()
}