export default {
    get media() {
        return process.env.MEDIA_DIR ?? "../media/photos";
    },
    get thumbnails() {
        return process.env.THUMBNAILS_DIR ?? "../media/thumbnails";
    },
    get backups() {
        return process.env.MEDIA_DIR ?? "../media/photos";
    },
    get syncInterval() {
        return process.env.SYNC_INTERVAL ?? 86400000;
    },
    get backupInterval() {
        return process.env.BACKUP_INTERVAL ?? 172800000;
    },
    get telegramToken() {
        return process.env.TELEGRAM_TOKEN ?? "";
    },
    get chatId() {
        return process.env.TELEGRAM_CHAT_ID ?? 0;
    },
    get geocodeAdmin3and4() {
        return (process.env.GEOCODE_ADMIN_3_AND_4 ?? false) === 'true';
    },
    get skipGeocode() {
        return (process.env.SKIP_GEOCODE ?? false) === 'true';
    },
    get hostThumbnails() {
        return (process.env.HOST_THUMBNAILS ?? false) === 'true';
    }
}
