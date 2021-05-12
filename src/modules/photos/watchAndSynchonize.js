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

const {Op} = seq;

// Todo
// add interval for sync files (set interval in config.json
// If syncing during a processMedia, make sure it doesn't get double added (dont allow double processmedia on same file)
// If some failure happens, retry after timeout, then post to telegram

const bigPic = await useDir(path.join(config.thumbnails, 'bigPic'));
const smallPic = await useDir(path.join(config.thumbnails, 'smallPic'));
const streamVid = await useDir(path.join(config.thumbnails, 'streamVid'));
const vidPoster = await useDir(path.join(config.thumbnails, 'vidPoster'));
const smallVidPoster = await useDir(path.join(config.thumbnails, 'smallVidPoster'));
const temp = await useDir(path.join(config.thumbnails, 'temp'));

//test:
// await processMedia('./photos/IMG_20210510_230607.jpg');

// await processMedia('./photos/VID_20210510_224246.mp4');

export async function watchAndSynchronize() {
    await syncFiles();

    console.log("Watching", config.media);
    fs.watch(config.media, async (eventType, filename) => {
        if (eventType === 'rename') {
            let changedFile = path.join(config.media, filename);
            if (await checkFileExists(changedFile)) {
                await waitSleep(600);
                let files = await getFilesRecursive(changedFile);
                for (let file of files)
                    await processMedia(file);
            } else {
                let success = await removeMedia(changedFile);
                console.log("Remove file processed", filename, "success?", success);
            }
        }
    });
}

async function syncFiles() {
    // Sync files
    let files = await getFilesRecursive(config.media);
    await Promise.all(files.map(processIfNeeded));

    files = await getFilesRecursive(config.media);
    let count = await MediaItem.count();
    if (files.length !== count) {
        let names = files.map(f => path.basename(f));
        // Find and remove all database entries that don't have an associated file
        let mismatch = await MediaItem.findAll({
            where: {filename: {[Op.notIn]: names,}}
        });
        for (let item of mismatch) {
            console.log(`Deleting ${item.filename} from DB`)
            await item.destroy();
        }
    }
}

async function processIfNeeded(filePath) {
    let processed = await isProcessed(filePath);
    if (!processed) {
        await processMedia(filePath);
    }
}

async function isProcessed(filePath) {
    let filename = path.basename(filePath);
    let item = await MediaItem.findOne({where: {filename}});
    if (!item)
        return false;

    let type = getFileType(filePath);
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

async function processMedia(filePath) {
    console.log("Processing media", filePath);
    let type = getFileType(filePath);

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
        await resizeImage({input: filePath, output: big, height,});
        await resizeImage({input: filePath, output: small, height: smallHeight,});
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
    return true;
}

async function removeMedia(filePath) {
    let type = getFileType(filePath);
    try {
        let filename = path.basename(filePath);
        let item = await MediaItem.findOne({where: {filename}});
        await item.destroy();

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
        console.warn("Remove error", e);
        return false;
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
    return mimeType.split('/')[0];
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
