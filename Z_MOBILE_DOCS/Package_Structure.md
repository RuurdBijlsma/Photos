### Part 3: Recommended Package Structure

```text
app/src/main/java/dev/ruurd/photos/
├── core/
│   ├── common/             # AppDispatchers, MediaRefreshCoordinator, Extensions
│   ├── database/           # Room Database, TypeConverters
│   │   ├── dao/            # MediaDao, DeviceFolderDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, DeviceFolderEntity, SyncStatus, TimelineMediaProjection, AlbumEntity, UploadQueueEntity
│   ├── network/            # OkHttp, Retrofit (Protobuf Lite + kotlinx.serialization), CookieJar, SessionManager, NetworkMonitor
│   │   ├── api/            # PhotosApiService
│   │   ├── auth/           # AuthAuthenticator
│   │   ├── cookie/         # PersistentCookieJar implementation
│   │   ├── discovery/      # LanServerScanner
│   │   ├── dto/            # JSON DTOs (AuthDtos, MediaItemDtos)
│   │   ├── interceptor/    # DynamicBaseUrlInterceptor
│   │   └── tus/            # Resumable TUS upload client
│   ├── image/              # Coil ImageLoader, LibavifDecoder, MediaStoreThumbnailFetcher, FullResFileCache, ShareCache
│   ├── permissions/        # MediaPermissions (full vs. partial access + ACCESS_MEDIA_LOCATION)
│   ├── navigation/         # NavHost routes (MainScreen root + pushed detail/settings routes), ViewerOverlay & ViewerSource contract
│   └── ui/                 # Shared UI components (Theme lives in ui/theme/)
│
├── data/                   # Repositories & Data Sources
│   ├── repository/
│   │   ├── MediaRepositoryImpl.kt
│   │   ├── MediaDetailsRepositoryImpl.kt
│   │   ├── DeviceFolderRepositoryImpl.kt
│   │   ├── AlbumRepositoryImpl.kt
│   │   ├── BackupRepositoryImpl.kt
│   │   └── AuthRepositoryImpl.kt
│   └── datasource/
│       └── local/          # MediaStoreDataSource, LocalMediaMapper, LocalExifReader
│
├── domain/                 # Pure Kotlin business logic & UseCases
│   ├── model/              # MediaAccess, DeviceFolderRules, PassOneDeduplication, ScanGuards, TimelineLayout, MediaDetails
│   ├── repository/         # Repository Interfaces (AuthRepository, MediaRepository, DeviceFolderRepository, LocalMediaStore, ScanStateStore)
│   └── usecase/
│       ├── CalculateTimelineLayoutUseCase.kt
│       ├── ObserveTimelineUseCase.kt
│       ├── SyncTimelineUseCase.kt
│       ├── ScanLocalMediaUseCase.kt
│       ├── RotateMediaUseCase.kt
│       └── EnqueueBackupUseCase.kt
│
├── work/                   # WorkManager background workers
│   ├── UploadWorker.kt     # TUS auto-upload execution (Constrained WorkManager / Foreground or UIDT for manual)
│   └── LocalScanWorker.kt  # Periodic/Event-driven MediaStore reconciliation
│
├── ui/
│   └── theme/              # Color.kt, Shape.kt, Theme.kt, Type.kt (M3 Dynamic Color + LocalSemanticColors)
│
└── features/               # Presentation layer (Compose UI + ViewModels)
    ├── auth/               # Login screen, Server URL & LAN discovery
    ├── main/               # MainScreen shell, FloatingBottomNav, PhotosTopBar, AvatarMenuSheet, PullToRefreshState
    ├── timeline/           # Justified row gallery view
    │   └── components/     # TimelineScrubber
    ├── viewer/             # Media viewer (HorizontalPager, Telephoto, Media3, Info Sheet)
    │   └── components/     # ZoomablePhotoPage, VideoPlayerPage, MediaInfoBottomSheet, PlaybackSpeedDialog
    ├── albums/             # Albums listing, Device Folders ("On this device"), & album detail views
    └── backup/             # Backup settings & status
```
