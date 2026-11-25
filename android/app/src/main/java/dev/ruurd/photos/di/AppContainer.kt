package dev.ruurd.photos.di

import android.app.ActivityManager
import android.content.Context
import android.content.pm.ApplicationInfo
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import coil3.ImageLoader
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.DefaultAppDispatchers
import dev.ruurd.photos.core.common.DefaultMediaFileSharer
import dev.ruurd.photos.core.common.DefaultMediaRefreshCoordinator
import dev.ruurd.photos.core.common.MediaFileSharer
import dev.ruurd.photos.core.common.MediaRefreshCoordinator
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.dao.DeviceFolderDao
import dev.ruurd.photos.core.database.dao.MediaDao
import dev.ruurd.photos.core.image.FullResFileCache
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.core.image.MemoryCacheKeyRegistry
import dev.ruurd.photos.core.image.PhotosImageLoader
import dev.ruurd.photos.core.image.ShareCache
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.auth.AuthAuthenticator
import dev.ruurd.photos.core.network.cookie.PersistentCookieJar
import dev.ruurd.photos.core.network.discovery.LanServerScanner
import dev.ruurd.photos.core.network.interceptor.DynamicBaseUrlInterceptor
import dev.ruurd.photos.core.state.BootstrapState
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoReader
import dev.ruurd.photos.data.datasource.local.LocalMediaInfoSource
import dev.ruurd.photos.data.datasource.local.MediaStoreDataSource
import dev.ruurd.photos.data.repository.AuthRepositoryImpl
import dev.ruurd.photos.data.repository.DeviceFolderRepositoryImpl
import dev.ruurd.photos.data.repository.MediaDetailsRepositoryImpl
import dev.ruurd.photos.data.repository.MediaRepositoryImpl
import dev.ruurd.photos.data.repository.TimelineProjectionStore
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.repository.DeviceFolderRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
import dev.ruurd.photos.domain.repository.MediaDetailsRepository
import dev.ruurd.photos.domain.repository.MediaRepository
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.domain.usecase.ObserveTimelineUseCase
import dev.ruurd.photos.domain.usecase.ScanLocalMediaUseCase
import dev.ruurd.photos.domain.usecase.SyncTimelineUseCase
import dev.ruurd.photos.features.auth.AuthViewModel
import dev.ruurd.photos.features.main.MainViewModel
import dev.ruurd.photos.features.timeline.TimelineViewModel
import dev.ruurd.photos.features.viewer.MediaViewerViewModel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.Dispatcher
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Converter
import retrofit2.Retrofit
import retrofit2.converter.kotlinx.serialization.asConverterFactory
import retrofit2.converter.protobuf.ProtoConverterFactory
import retrofit2.converter.scalars.ScalarsConverterFactory
import java.util.concurrent.TimeUnit

interface AppContainer {
    val dispatchers: AppDispatchers
    val database: AppDatabase
    val mediaDao: MediaDao
    val deviceFolderDao: DeviceFolderDao
    val sessionManager: SessionManager
    val cookieJar: PersistentCookieJar
    val okHttpClient: OkHttpClient
    val photosApiService: PhotosApiService
    val authRepository: AuthRepository
    val mediaRepository: MediaRepository
    val deviceFolderRepository: DeviceFolderRepository
    val localMediaStore: LocalMediaStore
    val scanLocalMediaUseCase: ScanLocalMediaUseCase
    val syncTimelineUseCase: SyncTimelineUseCase
    val observeTimelineUseCase: ObserveTimelineUseCase
    val calculateTimelineLayoutUseCase: CalculateTimelineLayoutUseCase
    val networkMonitor: NetworkMonitor
    val lanServerScanner: LanServerScanner
    val imageLoader: ImageLoader
    val memoryCacheKeyRegistry: MemoryCacheKeyRegistry
    val timelineProjectionStore: TimelineProjectionStore
    val bootstrapState: BootstrapState
    val mediaDetailsRepository: MediaDetailsRepository
    val fullResFileCache: FullResFileStore
    val shareCache: ShareCache
    val localMediaInfoReader: LocalMediaInfoSource
    val mediaFileSharer: MediaFileSharer

    fun authViewModelFactory(): ViewModelProvider.Factory
    fun mainViewModelFactory(): ViewModelProvider.Factory
    fun timelineViewModelFactory(): ViewModelProvider.Factory
    fun mediaViewerViewModelFactory(initialMediaId: String): ViewModelProvider.Factory
}

class DefaultAppContainer(private val context: Context) : AppContainer {

    private val applicationScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override val dispatchers: AppDispatchers by lazy {
        DefaultAppDispatchers()
    }

    override val database: AppDatabase by lazy {
        AppDatabase.create(context)
    }

    override val mediaDao: MediaDao by lazy {
        database.mediaDao()
    }

    override val deviceFolderDao: DeviceFolderDao by lazy {
        database.deviceFolderDao()
    }

    override val sessionManager: SessionManager by lazy {
        SessionManager(
            context = context,
            cookieJar = cookieJar,
            scope = applicationScope,
            onClearSessionData = {
                bootstrapState.reset()
                withContext(dispatchers.io) {
                    mediaRepository.clearAll()
                    imageLoader.memoryCache?.clear()
                    imageLoader.diskCache?.clear()
                    timelineProjectionStore.clear()
                    memoryCacheKeyRegistry.clear()
                    mediaDetailsRepository.clear()
                    fullResFileCache.clear()
                    shareCache.clear()
                }
            },
        )
    }

    override val cookieJar: PersistentCookieJar by lazy {
        PersistentCookieJar(context)
    }

    private val directClient: OkHttpClient by lazy {
        OkHttpClient.Builder()
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(15, TimeUnit.SECONDS)
            .build()
    }

    override val okHttpClient: OkHttpClient by lazy {
        val dispatcher = Dispatcher().apply {
            maxRequests = 32
            maxRequestsPerHost = 16
        }

        val dynamicBaseUrlInterceptor = DynamicBaseUrlInterceptor(sessionManager)
        val authenticator = AuthAuthenticator(cookieJar, sessionManager, applicationScope)

        OkHttpClient.Builder()
            .dispatcher(dispatcher)
            .cookieJar(cookieJar)
            .authenticator(authenticator)
            .addInterceptor(dynamicBaseUrlInterceptor)
            .apply {
                // This client also serves Coil, so request logging in a release build would add two
                // log lines per thumbnail.
                if (isDebuggable()) {
                    addInterceptor(
                        HttpLoggingInterceptor().apply {
                            level = HttpLoggingInterceptor.Level.BASIC
                        },
                    )
                }
            }
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()
    }

    override val photosApiService: PhotosApiService by lazy {
        val builder = Retrofit.Builder()
            .baseUrl("http://localhost:5272/") // DynamicBaseUrlInterceptor rewrites this
            .client(okHttpClient)
        photosApiConverterFactories().forEach { builder.addConverterFactory(it) }
        builder.build().create(PhotosApiService::class.java)
    }

    override val networkMonitor: NetworkMonitor by lazy {
        NetworkMonitor(context)
    }

    override val lanServerScanner: LanServerScanner by lazy {
        LanServerScanner(
            networkMonitor = networkMonitor,
            directClient = directClient,
            dispatchers = dispatchers,
        )
    }

    override val authRepository: AuthRepository by lazy {
        AuthRepositoryImpl(
            apiService = photosApiService,
            sessionManager = sessionManager,
            cookieJar = cookieJar,
            directClient = directClient,
            lanServerScanner = lanServerScanner,
            dispatchers = dispatchers,
            applicationScope = applicationScope,
        )
    }

    override val mediaRepository: MediaRepository by lazy {
        MediaRepositoryImpl(
            database = database,
            mediaDao = mediaDao,
            dispatchers = dispatchers,
        )
    }

    override val deviceFolderRepository: DeviceFolderRepository by lazy {
        DeviceFolderRepositoryImpl(deviceFolderDao = deviceFolderDao)
    }

    override val localMediaStore: LocalMediaStore by lazy {
        MediaStoreDataSource(context)
    }

    override val mediaDetailsRepository: MediaDetailsRepository by lazy {
        MediaDetailsRepositoryImpl(
            apiService = photosApiService,
            scope = applicationScope,
            dispatchers = dispatchers,
        )
    }

    override val fullResFileCache: FullResFileCache by lazy {
        FullResFileCache(
            context = context,
            okHttpClient = okHttpClient,
            sessionManager = sessionManager,
            scope = applicationScope,
            dispatchers = dispatchers,
        )
    }

    override val shareCache: ShareCache by lazy {
        ShareCache(context = context, dispatchers = dispatchers)
    }

    override val localMediaInfoReader: LocalMediaInfoReader by lazy {
        LocalMediaInfoReader(context = context, dispatchers = dispatchers)
    }

    override val mediaFileSharer: MediaFileSharer by lazy {
        DefaultMediaFileSharer(
            context = context,
            shareCache = shareCache,
            fullResFileCache = fullResFileCache,
            okHttpClient = okHttpClient,
            sessionManager = sessionManager,
            dispatchers = dispatchers,
        )
    }

    private val mediaRefreshCoordinator: MediaRefreshCoordinator by lazy {
        DefaultMediaRefreshCoordinator(
            mediaDetailsRepository = mediaDetailsRepository,
            fullResFileCache = fullResFileCache,
            scope = applicationScope,
        )
    }

    override val scanLocalMediaUseCase: ScanLocalMediaUseCase by lazy {
        ScanLocalMediaUseCase(
            localMediaStore = localMediaStore,
            mediaRepository = mediaRepository,
            deviceFolderRepository = deviceFolderRepository,
            scanStateStore = sessionManager,
            dispatchers = dispatchers,
        )
    }

    override val syncTimelineUseCase: SyncTimelineUseCase by lazy {
        SyncTimelineUseCase(
            apiService = photosApiService,
            mediaRepository = mediaRepository,
            sessionManager = sessionManager,
            dispatchers = dispatchers,
            mediaRefreshCoordinator = mediaRefreshCoordinator,
        )
    }

    override val observeTimelineUseCase: ObserveTimelineUseCase by lazy {
        ObserveTimelineUseCase(
            mediaRepository = mediaRepository,
            dispatchers = dispatchers,
            timelineProjectionStore = timelineProjectionStore,
        )
    }

    override val calculateTimelineLayoutUseCase: CalculateTimelineLayoutUseCase by lazy {
        val activityManager = context.getSystemService(ActivityManager::class.java)
        CalculateTimelineLayoutUseCase(
            maxThumbnailHeightPx = if (activityManager?.isLowRamDevice == true) 480 else 720,
        )
    }

    private fun isDebuggable(): Boolean = (context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0

    override val memoryCacheKeyRegistry: MemoryCacheKeyRegistry by lazy {
        MemoryCacheKeyRegistry()
    }

    override val timelineProjectionStore: TimelineProjectionStore by lazy {
        TimelineProjectionStore(
            mediaRepository = mediaRepository,
            scope = applicationScope,
            dispatchers = dispatchers,
        )
    }

    override val bootstrapState: BootstrapState by lazy {
        // Reached only after `SessionManager` loaded its persisted state, since that gates auth.
        BootstrapState(
            needsBootstrap = sessionManager.latestSyncTimestamp.value <= 0L &&
                !sessionManager.hasScanSignature.value,
        )
    }

    override val imageLoader: ImageLoader by lazy {
        PhotosImageLoader.create(context, okHttpClient)
    }

    override fun authViewModelFactory(): ViewModelProvider.Factory = viewModelFactory {
        initializer { AuthViewModel(authRepository = authRepository) }
    }

    override fun mainViewModelFactory(): ViewModelProvider.Factory = viewModelFactory {
        initializer {
            MainViewModel(
                authRepository = authRepository,
                syncTimelineUseCase = syncTimelineUseCase,
                scanLocalMediaUseCase = scanLocalMediaUseCase,
                localMediaStore = localMediaStore,
                bootstrapState = bootstrapState,
                networkMonitor = networkMonitor,
            )
        }
    }

    override fun timelineViewModelFactory(): ViewModelProvider.Factory = viewModelFactory {
        initializer {
            TimelineViewModel(
                observeTimelineUseCase = observeTimelineUseCase,
                calculateTimelineLayoutUseCase = calculateTimelineLayoutUseCase,
                dispatchers = dispatchers,
                isBootstrapping = bootstrapState.isBootstrapping,
            )
        }
    }

    override fun mediaViewerViewModelFactory(
        initialMediaId: String,
    ): ViewModelProvider.Factory = viewModelFactory {
        initializer {
            MediaViewerViewModel(
                initialMediaId = initialMediaId,
                timelineProjectionStore = timelineProjectionStore,
                mediaRepository = mediaRepository,
                mediaDetailsRepository = mediaDetailsRepository,
                fullResFileCache = fullResFileCache,
                localMediaInfoReader = localMediaInfoReader,
                mediaFileSharer = mediaFileSharer,
                dispatchers = dispatchers,
                mediaRefreshCoordinator = mediaRefreshCoordinator,
            )
        }
    }
}

/**
 * kotlinx-serialization claims every type and throws for the protobuf responses, so it must stay
 * last; [retrofit2.Retrofit] only asks the factories in order.
 */
internal fun photosApiConverterFactories(): List<Converter.Factory> = listOf(
    ScalarsConverterFactory.create(),
    ProtoConverterFactory.create(),
    Json { ignoreUnknownKeys = true }.asConverterFactory("application/json".toMediaType()),
)
