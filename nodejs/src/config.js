export default {
    "media": process.env.MEDIA_DIR ?? "/photos",
    "thumbnails": process.env.THUMBNAILS_DIR ?? "/thumbnails",
    "syncInterval": process.env.SYNC_INTERVAL ?? 86400000,
    "backupInterval": process.env.BACKUP_INTERVAL ?? 172800000,
    "telegramToken": process.env.TELEGRAM_TOKEN ?? "",
    "chatId": process.env.TELEGRAM_CHAT_ID ?? 0,
    "geocodeAdmin3and4": process.env.GEOCODE_ADMIN_3_AND_4 ?? false,
    "skipGeocode": process.env.SKIP_GEOCODE ?? false,
    "hostThumbnails": process.env.HOST_THUMBNAILS ?? false
}
