import classify from "./classify.js";
import {resizeImage, transcode, videoScreenshot} from "./transcode.js";
import {getExif, probeVideo} from "./exif.js";
import fs from 'fs'
import path from "path";
import mime from "mime-types";
import config from "../../../res/photos/config.json";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import {insertMediaItem} from "../../database/models/photos/mediaUtils.js";
import seq from "sequelize";

import TelegramBot from "node-telegram-bot-api";

const {Op} = seq;

// Todo
// rename all thumbnails to ${id}.webp ${id}.webm etc...,then static serve the files instead of requiring a database query
// Fix memory usage (maybe just reduce batch size, maybe try fix something with tensorflow)


const bot = new TelegramBot(config.telegramToken, {polling: false});
const bigPic = await useDir(path.join(config.thumbnails, 'bigPic'));
const smallPic = await useDir(path.join(config.thumbnails, 'smallPic'));
const streamVid = await useDir(path.join(config.thumbnails, 'streamVid'));
const vidPoster = await useDir(path.join(config.thumbnails, 'vidPoster'));
const smallVidPoster = await useDir(path.join(config.thumbnails, 'smallVidPoster'));
const temp = await useDir(path.join(config.thumbnails, 'temp'));
const processJobs = new Set();


export async function watchAndSynchronize() {
    await singleInstance(syncFiles);
    setInterval(async () => {
        await singleInstance(syncFiles);
    }, config.syncInterval);

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
                if (ext === '.jpeg' || ext === '.jpg' || ext === '.mp4')
                    await singleInstance(removeMedia, changedFile);
                else
                    // Deleted item might be a folder, sync to make sure the files get removed
                    await singleInstance(syncFiles);
            }
        }
    });
}

async function syncFiles() {
    console.log("Syncing...");
    // Sync files: add thumbnails and database entries for files in media directory
    let files = await getFilesRecursive(config.media);
    let batchSize = 50;
    for (let i = 0; i < files.length; i += batchSize) {
        let slice = files.slice(i, i + batchSize);
        await Promise.all(slice.map(processIfNeeded));
        console.log(`Processed [${i + batchSize} / ${files.length}] photos in sync job`);
    }

    // Find and remove all database entries that don't have an associated file
    files = await getFilesRecursive(config.media);
    let count = await MediaItem.count();
    if (files.length !== count) {
        let names = files.map(f => path.basename(f));
        let mismatch = await MediaItem.findAll({
            where: {filename: {[Op.notIn]: names,}}
        });
        for (let item of mismatch) {
            console.log(`Deleting ${item.filename} from DB`)
            await item.destroy();
        }
    }

    const cleanThumbDir = async dir => {
        let dirFiles = await fs.promises.readdir(dir);
        await Promise.all(dirFiles.map(f => deleteThumbIfAllowed(
            path.join(dir, f),
            files.map(f => path.basename(f))
        )));
    }
    // Delete all thumbnail files when the original file doesn't exist anymore
    await Promise.all([bigPic, smallPic, smallVidPoster, streamVid, vidPoster, temp].map(cleanThumbDir));
}

// Delete is allowed when the original photo isn't in the database anymore
async function deleteThumbIfAllowed(thumbPath, files) {
    let thumbFile = path.basename(thumbPath);
    let filename = thumbFile.substr(0, thumbFile.length - path.extname(thumbFile).length);
    if (!files.includes(filename)) {
        await fs.promises.unlink(thumbPath);
        console.log("Deleted", thumbPath, "original file", filename, "isn't available anymore")
    }
}

async function processIfNeeded(filePath) {
    let processed = await isProcessed(filePath);
    if (!processed)
        await singleInstance(processMedia, filePath);
}

async function isProcessed(filePath) {
    let filename = path.basename(filePath);
    let type = getFileType(filePath);
    if (type === false) return true;

    let item = await MediaItem.findOne({where: {filename}});
    if (!item)
        return false;

    let files = [];
    if (type === 'image') {
        let {big, small} = getPaths(filePath);
        files.push(big, small);
    } else if (type === 'video') {
        let {webm, poster, smallPoster} = getPaths(filePath);
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
        let type = getFileType(filePath);
        if (type === false) return;
        let filename = path.basename(filePath);

        let alreadyInDb = await MediaItem.findOne({where: {filename}});
        if (alreadyInDb)
            await alreadyInDb.destroy();

        let dbData, thumbSmallRel, thumbBigRel, webmRel;
        if (type === 'image') {
            let labels = await classify(filePath);
            let metadata = await getExif(filePath);
            let height = Math.min(metadata.height, 1440);
            let smallHeight = Math.min(metadata.height, 500);
            let {big, small} = getPaths(filePath);
            thumbSmallRel = path.relative(config.thumbnails, small);
            thumbBigRel = path.relative(config.thumbnails, big);
            webmRel = null;
            let orientation = metadata.exif.Orientation ?? 1;
            await resizeImage({input: filePath, orientation, output: big, height,});
            await resizeImage({input: filePath, orientation, output: small, height: smallHeight,});
            dbData = {
                filename,
                labels,
                ...metadata,
            }
        } else if (type === 'video') {
            let metadata = await probeVideo(filePath);
            let height = Math.min(metadata.height, 1080);
            let smallHeight = Math.min(metadata.height, 500);
            let {webm, poster, smallPoster, classifyPoster} = getPaths(filePath);
            thumbSmallRel = path.relative(config.thumbnails, smallPoster);
            thumbBigRel = path.relative(config.thumbnails, poster);
            webmRel = path.relative(config.thumbnails, webm);
            await transcode({input: filePath, output: webm, height});
            await videoScreenshot({input: webm, output: classifyPoster, height});
            await resizeImage({input: classifyPoster, output: poster, height: height,});
            await resizeImage({input: classifyPoster, output: smallPoster, height: smallHeight,});
            let labels = await classify(classifyPoster);
            await fs.promises.unlink(classifyPoster);
            dbData = {
                filename,
                labels,
                ...metadata,
            }
        }
        let fullRel = path.relative(config.media, filePath);
        await insertMediaItem({
            type: dbData.type,
            subType: dbData.subType,
            filename,
            filePath: fullRel,
            smallThumbPath: thumbSmallRel,
            bigThumbPath: thumbBigRel,
            webmPath: webmRel,
            width: dbData.width,
            height: dbData.height,
            durationMs: dbData.duration,
            bytes: dbData.size,
            createDate: dbData.createDate,
            exif: dbData.exif,
            classifications: dbData.labels,
            location: dbData.gps,
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
        await item?.destroy?.();

        if (type === 'image') {
            let {big, small} = getPaths(filePath);
            console.log("Removing media", {big, small});
            await fs.promises.unlink(big);
            await fs.promises.unlink(small);
        } else if (type === 'video') {
            let {webm, poster, smallPoster} = getPaths(filePath);
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

function getPaths(filePath) {
    let filename = path.basename(filePath);
    let big = path.join(bigPic, filename + '.webp');
    let small = path.join(smallPic, filename + '.webp');
    let webm = path.join(streamVid, filename + '.webm');
    let poster = path.join(vidPoster, filename + '.webp');
    let smallPoster = path.join(smallVidPoster, filename + '.webp');
    let classifyPoster = path.join(temp, filename + '.jpeg');
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
