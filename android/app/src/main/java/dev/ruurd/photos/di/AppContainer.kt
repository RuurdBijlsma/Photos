package dev.ruurd.photos.di

import android.app.ActivityManager
import android.content.Context
import coil3.ImageLoader
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.core.common.DefaultAppDispatchers
import dev.ruurd.photos.core.database.AppDatabase
import dev.ruurd.photos.core.database.dao.DeviceFolderDao
import dev.ruurd.photos.core.database.dao.MediaDao
import dev.ruurd.photos.core.image.PhotosImageLoader
import dev.ruurd.photos.core.network.NetworkMonitor
import dev.ruurd.photos.core.network.SessionManager
import dev.ruurd.photos.core.network.api.PhotosApiService
import dev.ruurd.photos.core.network.auth.AuthAuthenticator
import dev.ruurd.photos.core.network.cookie.PersistentCookieJar
import dev.ruurd.photos.core.network.interceptor.DynamicBaseUrlInterceptor
import dev.ruurd.photos.data.datasource.local.MediaStoreDataSource
import dev.ruurd.photos.data.repository.AuthRepositoryImpl
import dev.ruurd.photos.data.repository.DeviceFolderRepositoryImpl
import dev.ruurd.photos.data.repository.MediaRepositoryImpl
import dev.ruurd.photos.domain.repository.AuthRepository
import dev.ruurd.photos.domain.repository.DeviceFolderRepository
import dev.ruurd.photos.domain.repository.LocalMediaStore
import dev.ruurd.photos.domain.repository.MediaRepository
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.domain.usecase.ObserveTimelineUseCase
import dev.ruurd.photos.domain.usecase.ScanLocalMediaUseCase
import dev.ruurd.photos.domain.usecase.SyncTimelineUseCase
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.withContext
import okhttp3.Dispatcher
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
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
    val imageLoader: ImageLoader
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
            scope = applicationScope,
            onClearSessionData = {
                withContext(dispatchers.io) {
                    mediaRepository.clearAll()
                    imageLoader.memoryCache?.clear()
                    imageLoader.diskCache?.clear()
                }
            }
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

        val logging = HttpLoggingInterceptor().apply {
            level = HttpLoggingInterceptor.Level.BASIC
        }

        val dynamicBaseUrlInterceptor = DynamicBaseUrlInterceptor(sessionManager)
        val authenticator = AuthAuthenticator(cookieJar, sessionManager, applicationScope)

        OkHttpClient.Builder()
            .dispatcher(dispatcher)
            .cookieJar(cookieJar)
            .authenticator(authenticator)
            .addInterceptor(dynamicBaseUrlInterceptor)
            .addInterceptor(logging)
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()
    }

    override val photosApiService: PhotosApiService by lazy {
        Retrofit.Builder()
            .baseUrl("http://localhost:5272/") // DynamicBaseUrlInterceptor rewrites this
            .client(okHttpClient)
            .addConverterFactory(ScalarsConverterFactory.create())
            .addConverterFactory(ProtoConverterFactory.create())
            .build()
            .create(PhotosApiService::class.java)
    }

    override val authRepository: AuthRepository by lazy {
        AuthRepositoryImpl(
            apiService = photosApiService,
            sessionManager = sessionManager,
            cookieJar = cookieJar,
            directClient = directClient,
            dispatchers = dispatchers,
            applicationScope = applicationScope
        )
    }

    override val mediaRepository: MediaRepository by lazy {
        MediaRepositoryImpl(
            database = database,
            mediaDao = mediaDao,
            dispatchers = dispatchers
        )
    }

    override val deviceFolderRepository: DeviceFolderRepository by lazy {
        DeviceFolderRepositoryImpl(deviceFolderDao = deviceFolderDao)
    }

    override val localMediaStore: LocalMediaStore by lazy {
        MediaStoreDataSource(context.contentResolver)
    }

    override val scanLocalMediaUseCase: ScanLocalMediaUseCase by lazy {
        ScanLocalMediaUseCase(
            localMediaStore = localMediaStore,
            mediaRepository = mediaRepository,
            deviceFolderRepository = deviceFolderRepository,
            dispatchers = dispatchers
        )
    }

    override val syncTimelineUseCase: SyncTimelineUseCase by lazy {
        SyncTimelineUseCase(
            apiService = photosApiService,
            mediaRepository = mediaRepository,
            sessionManager = sessionManager,
            dispatchers = dispatchers
        )
    }

    override val observeTimelineUseCase: ObserveTimelineUseCase by lazy {
        ObserveTimelineUseCase(mediaRepository = mediaRepository, dispatchers = dispatchers)
    }

    override val calculateTimelineLayoutUseCase: CalculateTimelineLayoutUseCase by lazy {
        val activityManager = context.getSystemService(ActivityManager::class.java)
        CalculateTimelineLayoutUseCase(
            maxThumbnailHeightPx = if (activityManager?.isLowRamDevice == true) 480 else 720
        )
    }

    override val networkMonitor: NetworkMonitor by lazy {
        NetworkMonitor(context)
    }

    override val imageLoader: ImageLoader by lazy {
        PhotosImageLoader.create(context, okHttpClient)
    }
}