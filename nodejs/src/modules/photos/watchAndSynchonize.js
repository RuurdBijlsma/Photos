import classify from "./classify.js";
import { resizeImage, transcode, videoScreenshot } from "./transcode.js";
import { getExif, probeVideo } from "./exif.js";
import fs from 'fs';
import path from "path";
import mime from "mime-types";
import config from '../../config.js'
import { Media } from "../../database/models/MediaModel.js";
import { deleteOldLogs, deleteOldZips, dropMedia, getUniqueId, insertMedia } from "../../database/models/mediaUtils.js";
import seq from "sequelize";
import TelegramBot from "node-telegram-bot-api";
import Database from "../../database/Database.js";
import { batchSize, checkFileExists, getToken, waitSleep } from "../../utils.js";
import { Blocked } from "../../database/models/BlockedModel.js";
import Clog from '../../Clog.js'

const console = new Clog('watcher');

const { Op } = seq;

// Make sure the thumbnails dir exists
await useDir(config.thumbnails);
console.log("USING BOT TOKEN", config.telegramToken, "CHAT ID", config.chatId);
const bot = new TelegramBot(config.telegramToken, { polling: false });
await useDir(config.media);
await useDir(config.thumbnails);
await useDir(config.backups);
export const uploadDir = await useDir(path.join(config.media, 'upload'));
export const zipDir = await useDir(path.join(config.thumbnails, 'zip'));
const tinyPic = await useDir(path.join(config.thumbnails, 'tiny'));
const smallPic = await useDir(path.join(config.thumbnails, 'small'));
const bigPic = await useDir(path.join(config.thumbnails, 'big'));
const streamVid = await useDir(path.join(config.thumbnails, 'webm'));
export const temp = await useDir(path.join(config.thumbnails, 'temp'));
const processJobs = new Set();


export async function watchAndSynchronize() {
    await deleteOldLogs();
    await deleteOldZips();
    if (process.platform !== 'win32')
        setInterval(async () => {
            await deleteOldLogs();
            await deleteOldZips();
            try {
                await Database.backup();
            } catch (e) {
                if (config.chatId !== 0)
                    await bot.sendMessage(config.chatId, `Couldn't backup database!\n\n${JSON.stringify(e)}`);
            }
        }, config.backupInterval);
    // else return console.warn("NOT WATCHING AND SYNCHRONIZING ON WINDOWS");

    console.log("Watching", config.media);
    global.dontWatch = false;
    fs.watch(config.media, async (eventType, filename) => {
        console.log("Watch fire! global.dontWatch = ", global.dontWatch);
        if(global.dontWatch) return;
        if (eventType === 'rename') {
            try {
                let changedFile = path.join(config.media, filename);
                if (await checkFileExists(changedFile)) {
                    await waitSleep(600);
                    let { files } = await getFilesRecursive(changedFile);
                    for (let file of files)
                        await singleInstance(processMedia, file);
                } else {
                    let ext = path.extname(changedFile)
                    if (ext !== '')
                        await singleInstance(removeMedia, changedFile);
                    else
                        // Deleted item might be a folder, sync to make sure the files get removed
                        await singleInstance(syncFiles);
                }
            } catch (e) {
                console.warn('watch error', e);
            }
        }
    });

    await singleInstance(syncFiles);
    setInterval(async () => {
        await singleInstance(syncFiles);
    }, config.syncInterval);
}

async function syncFiles() {
    console.log("Syncing...");
    // Sync files: add thumbnails and database entries for files in media directory
    let { files, videos, images } = await getFilesRecursive(config.media);
    let newFiles = [];
    console.log(`Checking ${files.length} files to see if they need to get processed. [BatchSize: ${batchSize}]`);
    for (let i = 0; i < images.length; i += batchSize) {
        let slice = images.slice(i, i + batchSize);
        console.log(`Processing images [${i}-${Math.min(images.length, i + batchSize)} / ${images.length}]`);
        newFiles.push(...await Promise.all(slice.map(processIfNeeded)));
    }
    console.log(`Processing ${videos.length} videos`)
    for (let i = 0; i < videos.length; i++) {
        console.log(`Processing video [${i + 1} / ${videos.length}]`)
        newFiles.push(await processIfNeeded(videos[i]));
    }
    newFiles = newFiles.filter(n => n !== false);
    console.log(`Sync has processed ${newFiles.length} new files`);
    files.push(...newFiles);

    // Find and remove all database entries that don't have an associated file
    let count = await Media.count();
    if (files.length !== count) {
        let names = files.map(f => path.basename(f));
        let toRemove = await Media.findAll({
            where: { filename: { [Op.notIn]: names, } }
        });
        for(let medium of toRemove){
            console.log("Dropping ", medium.filename)
            await dropMedia(medium.id);
        }
        // following function makes application hang when dropping many media items
        // await Promise.all(toRemove.map(i => dropMedia(i.id)));
    }

    // Delete all thumbnail files when there is no database entry for them
    const idToFile = {};
    const cleanThumbDir = async dir => {
        let dirFiles = await fs.promises.readdir(dir);
        await Promise.all(dirFiles.map(f => deleteThumbIfAllowed(
            path.join(dir, f),
            idToFile,
        )));
    }
    await Promise.all([bigPic, tinyPic, smallPic, streamVid, temp].map(cleanThumbDir));
}

// Delete is allowed when the original photo doesn't exist anymore
async function deleteThumbIfAllowed(thumbPath, idToFile = {}) {
    let thumbFile = path.basename(thumbPath);
    let id = thumbFile.substr(0, thumbFile.length - path.extname(thumbFile).length);
    if (!idToFile.hasOwnProperty(id))
        idToFile[id] = Media.findOne({ where: { id }, attributes: ['filename'] });
    let item = await idToFile[id];
    if (item === null) {
        await fs.promises.unlink(thumbPath);
        console.log("Deleted", thumbPath, "original file isn't available anymore")
    }
}

async function processIfNeeded(filePath) {
    let processed = await isProcessed(filePath);
    if (!processed) {
        await singleInstance(processMedia, filePath);
        return filePath;
    }
    return false;
}

async function isProcessed(filePath) {
    let filename = path.basename(filePath);
    let type = getFileType(filePath);
    if (type === false) return true;

    let item = await Media.findOne({ where: { filename } });
    if (!item) {
        let fullRel = path.relative(config.media, filePath);
        let hasFailed = await Blocked.findOne({ where: { filePath: fullRel } });
        if (hasFailed) {
            console.warn(`${filePath} will not be reprocessed, it has already failed before.`);
            return true;
        }
        console.log(`${filePath} not processed, reason: Not in DB`);
        return false;
    }
    const id = item.id;

    let files = [];
    if (type === 'image') {
        let { big, tiny, small } = getPaths(id);
        files.push(big, tiny, small);
    } else if (type === 'video') {
        let { webm, big, small, tiny } = getPaths(id);
        files.push(webm, big, small, tiny);
    }
    for (let file of files) {
        if (!await checkFileExists(file)) {
            console.log(`${filePath} not processed, reason: File don't exist: ${file}`);
            return false;
        }
        let stat = await fs.promises.stat(file);
        if (!stat.isFile()) {
            console.log(`${filePath} not processed, reason: File is not a file: ${file}`);
            return false;
        }
    }

    return true;
}

async function singleInstance(fun, param) {
    let id = JSON.stringify({ fun: fun.toString(), param });
    if (processJobs.hasOwnProperty(id)) {
        console.log("this function is already running with these args!", fun, { param });
        return processJobs[id];
    }
    processJobs[id] = fun(param);
    let result = await processJobs[id];
    delete processJobs[id];
    return result;
}

export async function processMedia(filePath, triesLeft = 2, transaction = null) {
    // console.log("Processing media", filePath);
    try {
        let fullRel = path.relative(config.media, filePath);
        const spreadTransaction = transaction ? { transaction } : {};
        const id = await getUniqueId();
        let type = getFileType(filePath);
        if (type === false) return;
        let filename = path.basename(filePath);

        let alreadyInDb = await Media.findOne({ where: { filename }, ...spreadTransaction });
        if (alreadyInDb)
            await dropMedia(alreadyInDb.id, transaction);

        let fileStat = await fs.promises.stat(filePath);
        if (fileStat.size === 0)
            return console.warn(`Skipping ${filePath}, file size is 0 bytes`);

        let metadata = {}, labels;
        if (type === 'image') {
            metadata = await getExif(filePath);
            labels = await classify(filePath);
            let height = Math.min(metadata.height, 1440);
            let smallHeight = Math.min(metadata.height, 500);
            let tinyHeight = Math.min(metadata.height, 260);
            let { tiny, small, big } = getPaths(id);
            let orientation = metadata.exif?.Orientation ?? 1;
            // let orientation = 1;
            await resizeImage({ input: filePath, orientation, output: big, height, });
            await resizeImage({ input: filePath, orientation, output: small, height: smallHeight, });
            await resizeImage({ input: filePath, orientation, output: tiny, height: tinyHeight, });
        } else if (type === 'video') {
            metadata = await probeVideo(filePath);
            let height = Math.min(metadata.height, 1080);
            let smallHeight = Math.min(metadata.height, 500);
            let tinyHeight = Math.min(metadata.height, 260);
            let { tiny, small, big, classifyPoster, webm } = getPaths(id);
            await transcode({ input: filePath, output: webm, height });
            await videoScreenshot({ input: webm, output: classifyPoster, height });
            await resizeImage({ input: classifyPoster, output: big, height: height, });
            await resizeImage({ input: classifyPoster, output: small, height: smallHeight, });
            await resizeImage({ input: classifyPoster, output: tiny, height: tinyHeight, });
            labels = await classify(classifyPoster);
            await fs.promises.unlink(classifyPoster);
        }
        await insertMedia({
            id,
            type,
            subType: metadata.subType,
            filename,
            filePath: fullRel,
            width: metadata.width,
            height: metadata.height,
            durationMs: metadata.duration,
            bytes: metadata.size,
            createDateString: metadata.createDate,
            exif: metadata.exif,
            classifications: labels,
            location: metadata.gps,
        }, transaction);
        return id;
    } catch (e) {
        if (triesLeft === 0) {
            if (config.chatId !== 0)
                await bot.sendMessage(config.chatId, `[Photos] Failed to process "${filePath
                    }"\n\n${JSON.stringify(e.message)}`);
            let type;
            try {
                type = getFileType(filePath);
            } catch (e) {
                type = 'none'
            }
            let fullRel = path.relative(config.media, filePath);
            if (!await Blocked.findOne({ where: { filePath: fullRel } }))
                await Blocked.create({
                    filePath: fullRel,
                    error: {
                        name: e.name,
                        message: e.message,
                    },
                    reason: 'error',
                    type,
                    id: await getToken(),
                });
            return false;
        } else {
            const waitTime = (3 - triesLeft) ** 2 * 5000;
            console.warn("Process media failed for", filePath, `RETRYING AFTER ${waitTime}ms...`);
            console.warn(e.message);
            await waitSleep(waitTime);
            return processMedia(filePath, triesLeft - 1, transaction);
        }
    }
}

async function removeMedia(filePath, triesLeft = 2) {
    try {
        let type = getFileType(filePath);
        if (type === false) return;
        let filename = path.basename(filePath);
        let item = await Media.findOne({ where: { filename } });
        if (item === null)
            return;
        const id = item.id;
        await dropMedia(id);

        if (type === 'image') {
            let { tiny, small, big } = getPaths(id);
            console.log("Removing media", { big, small });
            await fs.promises.unlink(tiny);
            await fs.promises.unlink(small);
            await fs.promises.unlink(big);
        } else if (type === 'video') {
            let { webm, tiny, small, big } = getPaths(id);
            console.log("Removing media", { webm, tiny, small, big });
            await fs.promises.unlink(tiny);
            await fs.promises.unlink(small);
            await fs.promises.unlink(big);
            await fs.promises.unlink(webm);
        }
        return true;
    } catch (e) {
        if (triesLeft === 0) {
            if (config.chatId !== 0)
                await bot.sendMessage(config.chatId, `[Photos] Failed to remove media, file path: "${filePath
                    }"\n\n${JSON.stringify(e)}`);
            return false;
        } else {
            const waitTime = (3 - triesLeft) ** 2 * 5000;
            console.warn("Remove media failed for", filePath, `RETRYING AFTER ${waitTime}ms...`);
            console.warn(e);
            await waitSleep(waitTime);
            return removeMedia(filePath, triesLeft - 1);
        }
    }
}

async function getFilesRecursive(filePath) {
    let files = [],
        videos = [],
        images = [];
    let fileStat = await fs.promises.stat(filePath);
    if (fileStat.isDirectory()) {
        let subFiles = await fs.promises.readdir(filePath);
        let subResults = await Promise.all(
            subFiles.map(file => path.join(filePath, file)).map(getFilesRecursive)
        );
        files.push(...subResults.flatMap(r => r.files));
        videos.push(...subResults.flatMap(r => r.videos));
        images.push(...subResults.flatMap(r => r.images));
    } else {
        files.push(filePath);
        if (getFileType(filePath) === 'video')
            videos.push(filePath);
        else images.push(filePath);
    }
    return { files, videos, images };
}

function getFileType(filePath) {
    let fileExt = path.extname(filePath);
    let mimeType = mime.lookup(fileExt);
    return mimeType === false ? mimeType : mimeType.split('/')[0];
}

export function getPaths(id) {
    let big = path.join(bigPic, id + '.webp');
    let tiny = path.join(tinyPic, id + '.webp');
    let small = path.join(smallPic, id + '.webp');
    let webm = path.join(streamVid, id + '.webm');
    let classifyPoster = path.join(temp, id + '.jpeg');
    return { big, tiny, small, webm, classifyPoster }
}

async function useDir(dir) {
    if (!await checkFileExists(dir))
        await fs.promises.mkdir(dir);
    return dir;
}
