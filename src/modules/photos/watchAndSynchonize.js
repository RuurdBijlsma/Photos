import classify from "./classify.js";
import {resizeImage, transcode, videoScreenshot} from "./transcode.js";
import {getExif, probeVideo} from "./exif.js";
import fs from 'fs'
import path from "path";
import mime from "mime-types";
import config from "../../../res/photos/config.json";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import {getUniqueId, insertMediaItem} from "../../database/models/photos/mediaUtils.js";
import seq from "sequelize";
import TelegramBot from "node-telegram-bot-api";

const {Op} = seq;

// Todo
// Prepare for huge number of files in media dir (try to have support for ~100k items)
// mainly look for stuff that happens regularly but gets slower when n items in folder is larger
// such as sync items


const bot = new TelegramBot(config.telegramToken, {polling: false});
const bigPic = await useDir(path.join(config.thumbnails, 'big'));
const smallPic = await useDir(path.join(config.thumbnails, 'small'));
const streamVid = await useDir(path.join(config.thumbnails, 'webm'));
const vidPoster = await useDir(path.join(config.thumbnails, 'poster'));
const smallVidPoster = await useDir(path.join(config.thumbnails, 'smallposter'));
const temp = await useDir(path.join(config.thumbnails, 'temp'));
const processJobs = new Set();


export async function watchAndSynchronize() {
    console.log("Watching", config.media);
    fs.watch(config.media, async (eventType, filename) => {
        if (eventType === 'rename') {
            let changedFile = path.join(config.media, filename);
            if (await checkFileExists(changedFile)) {
                await waitSleep(600);
                let files = await getFilesRecursive(changedFile);
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
    let files = await getFilesRecursive(config.media);
    let newFiles = [];
    let batchSize = 30;
    console.log(`Checking ${files.length} files to see if they need to get processed`);
    for (let i = 0; i < files.length; i += batchSize) {
        let slice = files.slice(i, i + batchSize);
        console.log(`Processing [${i}-${Math.min(files.length, i + batchSize)} / ${files.length}]`);
        newFiles.push(...await Promise.all(slice.map(processIfNeeded)));
    }
    newFiles = newFiles.filter(n => n !== false);
    console.log(`Sync has processed ${newFiles.length} new files`);
    files.push(...newFiles);

    // Find and remove all database entries that don't have an associated file
    let count = await MediaItem.count();
    if (files.length !== count) {
        let names = files.map(f => path.basename(f));
        await MediaItem.destroy({
            where: {filename: {[Op.notIn]: names,}}
        });
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
    await Promise.all([bigPic, smallPic, smallVidPoster, streamVid, vidPoster, temp].map(cleanThumbDir));
}

// Delete is allowed when the original photo doesn't exist anymore
async function deleteThumbIfAllowed(thumbPath, idToFile = {}) {
    let thumbFile = path.basename(thumbPath);
    let id = thumbFile.substr(0, thumbFile.length - path.extname(thumbFile).length);
    if (!idToFile.hasOwnProperty(id))
        idToFile[id] = MediaItem.findOne({where: {id}, attributes: ['filename']});
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

    let item = await MediaItem.findOne({where: {filename}});
    if (!item)
        return false;
    const id = item.id;

    let files = [];
    if (type === 'image') {
        let {big, small} = getPaths(id);
        files.push(big, small);
    } else if (type === 'video') {
        let {webm, poster, smallPoster} = getPaths(id);
        files.push(webm, poster, smallPoster);
    }
    for (let file of files) {
        if (!await checkFileExists(file))
            return false;
        let stat = await fs.promises.stat(file);
        if (!stat.isFile())
            return false;
    }

    return true;
}

async function singleInstance(fun, param) {
    let id = JSON.stringify({fun: fun.toString(), param});
    if (processJobs.hasOwnProperty(id)) {
        console.log("this function is already running with these args!", fun, {param});
        return processJobs[id];
    }
    processJobs[id] = fun(param);
    let result = await processJobs[id];
    delete processJobs[id];
    return result;
}

async function processMedia(filePath, triesLeft = 3) {
    // console.log("Processing media", filePath);
    try {
        const id = await getUniqueId();
        let type = getFileType(filePath);
        if (type === false) return;
        let filename = path.basename(filePath);

        let alreadyInDb = await MediaItem.findOne({where: {filename}});
        if (alreadyInDb)
            await alreadyInDb.destroy();

        let metadata, labels, thumbSmallRel, thumbBigRel, webmRel;
        if (type === 'image') {
            metadata = await getExif(filePath);
            labels = await classify(filePath);
            let height = Math.min(metadata.height, 1440);
            let smallHeight = Math.min(metadata.height, 500);
            let {big, small} = getPaths(id);
            thumbSmallRel = path.relative(config.thumbnails, small);
            thumbBigRel = path.relative(config.thumbnails, big);
            webmRel = null;
            let orientation = metadata.exif.Orientation ?? 1;
            await resizeImage({input: filePath, orientation, output: big, height,});
            await resizeImage({input: filePath, orientation, output: small, height: smallHeight,});
        } else if (type === 'video') {
            metadata = await probeVideo(filePath);
            let height = Math.min(metadata.height, 1080);
            let smallHeight = Math.min(metadata.height, 500);
            let {webm, poster, smallPoster, classifyPoster} = getPaths(id);
            thumbSmallRel = path.relative(config.thumbnails, smallPoster);
            thumbBigRel = path.relative(config.thumbnails, poster);
            webmRel = path.relative(config.thumbnails, webm);
            await transcode({input: filePath, output: webm, height});
            await videoScreenshot({input: webm, output: classifyPoster, height});
            await resizeImage({input: classifyPoster, output: poster, height: height,});
            await resizeImage({input: classifyPoster, output: smallPoster, height: smallHeight,});
            labels = await classify(classifyPoster);
            await fs.promises.unlink(classifyPoster);
        }
        let fullRel = path.relative(config.media, filePath);
        await insertMediaItem({
            id,
            type,
            subType: metadata.subType,
            filename,
            filePath: fullRel,
            smallThumbPath: thumbSmallRel,
            bigThumbPath: thumbBigRel,
            webmPath: webmRel,
            width: metadata.width,
            height: metadata.height,
            durationMs: metadata.duration,
            bytes: metadata.size,
            createDate: metadata.createDate,
            exif: metadata.exif,
            classifications: labels,
            location: metadata.gps,
        })
    } catch (e) {
        if (triesLeft === 0) {
            await bot.sendMessage(config.chatId, `[Photos] Failed to process media, file path: "${filePath}"`);
            await bot.sendMessage(config.chatId, JSON.stringify(e));
            return false;
        } else {
            const waitTime = (4 - triesLeft) ** 2 * 5000;
            console.warn("Process media failed for", filePath, `RETRYING AFTER ${waitTime}ms...`);
            console.warn(e);
            await waitSleep(waitTime);
            return processMedia(filePath, triesLeft - 1);
        }
    }
    return true;
}

async function removeMedia(filePath, triesLeft = 3) {
    try {
        let type = getFileType(filePath);
        if (type === false) return;
        let filename = path.basename(filePath);
        let item = await MediaItem.findOne({where: {filename}});
        const id = item.id;
        await item?.destroy?.();

        if (type === 'image') {
            let {big, small} = getPaths(id);
            console.log("Removing media", {big, small});
            await fs.promises.unlink(big);
            await fs.promises.unlink(small);
        } else if (type === 'video') {
            let {webm, poster, smallPoster} = getPaths(id);
            console.log("Removing media", {webm, poster, smallPoster});
            await fs.promises.unlink(webm);
            await fs.promises.unlink(poster);
            await fs.promises.unlink(smallPoster);
        }
        return true;
    } catch (e) {
        if (triesLeft === 0) {
            await bot.sendMessage(config.chatId, `[Photos] Failed to remove media, file path: "${filePath}"`);
            await bot.sendMessage(config.chatId, JSON.stringify(e));
            return false;
        } else {
            const waitTime = (4 - triesLeft) ** 2 * 5000;
            console.warn("Remove media failed for", filePath, `RETRYING AFTER ${waitTime}ms...`);
            console.warn(e);
            await waitSleep(waitTime);
            return removeMedia(filePath, triesLeft - 1);
        }
    }
}

async function getFilesRecursive(filePath) {
    let files = [];
    let fileStat = await fs.promises.stat(filePath);
    if (fileStat.isDirectory()) {
        for (let file of await fs.promises.readdir(filePath))
            files.push(...await getFilesRecursive(path.join(filePath, file)));
    } else {
        files.push(filePath);
    }
    return files;
}

function getFileType(filePath) {
    let fileExt = path.extname(filePath);
    let mimeType = mime.lookup(fileExt);
    return mimeType === false ? mimeType : mimeType.split('/')[0];
}

function getPaths(id) {
    let big = path.join(bigPic, id + '.webp');
    let small = path.join(smallPic, id + '.webp');
    let webm = path.join(streamVid, id + '.webm');
    let poster = path.join(vidPoster, id + '.webp');
    let smallPoster = path.join(smallVidPoster, id + '.webp');
    let classifyPoster = path.join(temp, id + '.jpeg');
    return {big, small, webm, poster, smallPoster, classifyPoster}
}

async function checkFileExists(file) {
    return fs.promises.access(file, fs.constants.F_OK)
        .then(() => true)
        .catch(() => false);
}

async function useDir(dir) {
    if (!await checkFileExists(dir))
        await fs.promises.mkdir(dir);
    return dir;
}

async function waitSleep(ms = 1000) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
