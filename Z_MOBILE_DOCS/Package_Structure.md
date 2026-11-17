### Part 3: Recommended Package Structure

```text
app/src/main/java/dev/ruurd/photos/
├── core/
│   ├── common/             # Dispatchers, MediaRefreshCoordinator, Extensions
│   ├── database/           # Room Database, TypeConverters
│   │   ├── dao/            # MediaDao, DeviceFolderDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, DeviceFolderEntity, AlbumEntity, UploadQueueEntity
│   ├── network/            # OkHttp, Retrofit (Protobuf Lite + kotlinx.serialization), CookieJar, SessionManager, NetworkMonitor
│   │   ├── api/            # PhotosApiService
│   │   ├── auth/           # AuthAuthenticator
│   │   ├── cookie/         # PersistentCookieJar implementation
│   │   ├── discovery/      # LanServerScanner
│   │   ├── dto/            # JSON DTOs (AuthDtos, MediaItemDtos)
│   │   └── tus/            # Resumable TUS upload client
│   ├── image/              # Coil ImageLoader, LibavifDecoder, MediaStoreThumbnailFetcher, FullResFileCache
│   ├── navigation/         # NavHost routes, ViewerOverlay & ViewerSource contract
│   └── ui/                 # Shared UI components, Theme (M3 Dynamic Color)
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
│   ├── model/              # MediaAccess, DeviceFolderRules, PassOneDeduplication, TimelineLayout, MediaDetails
│   ├── repository/         # Repository Interfaces
│   └── usecase/
│       ├── CalculateTimelineLayoutUseCase.kt
│       ├── ObserveTimelineUseCase.kt
│       ├── SyncTimelineUseCase.kt
│       ├── ScanLocalMediaUseCase.kt
│       ├── RotateMediaUseCase.kt
│       └── EnqueueBackupUseCase.kt
│
├── work/                   # WorkManager background workers
│   ├── UploadWorker.kt     # TUS auto-upload execution (Constrained WorkManager / Foreground for manual)
│   └── LocalScanWorker.kt  # Periodic/Event-driven MediaStore reconciliation
│
└── features/               # Presentation layer (Compose UI + ViewModels)
    ├── auth/               # Login screen, Server URL & LAN discovery
    ├── main/               # MainScreen shell, FloatingBottomNav, PhotosTopBar, AvatarMenuSheet
    ├── timeline/           # Justified row gallery view
    │   └── components/     # TimelineScrubber
    ├── viewer/             # Media viewer (HorizontalPager, Telephoto, Media3, Info Sheet)
    │   └── components/     # ZoomablePhotoPage, VideoPlayerPage, MediaInfoBottomSheet, PlaybackSpeedDialog
    ├── albums/             # Albums listing, Device Folders ("On this device"), & album detail views
    └── backup/             # Backup settings & status
```
