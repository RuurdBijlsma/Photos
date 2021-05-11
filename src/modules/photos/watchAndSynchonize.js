import classify from "./classify.js";
import {resize, transcode, videoScreenshot} from "./transcode.js";
import {getExif, probeVideo} from "./exif.js";
import fs from 'fs'
import path from "path";
import mime from "mime-types";
import config from "../../../res/photos/config.json";

// Todo
// Database stuff
// Add process all files that aren't processed (use db for this)
// (Repeatedly) check for any files in media folder that haven't been processed yet


const bigPic = await useDir(path.join(config.thumbnails, 'bigPic'));
const smallPic = await useDir(path.join(config.thumbnails, 'smallPic'));
const streamVid = await useDir(path.join(config.thumbnails, 'streamVid'));
const vidPoster = await useDir(path.join(config.thumbnails, 'vidPoster'));
const smallVidPoster = await useDir(path.join(config.thumbnails, 'smallVidPoster'));

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
    console.log(files);
}

async function processIfNeeded(filePath) {
    let processed = await isProcessed(filePath);
    if (!processed)
        await processMedia(filePath);
}

async function isProcessed(filePath) {
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

    if (type === 'image') {
        let labels = await classify(filePath);
        let metadata = await getExif(filePath);
        let height = Math.min(metadata.height, 1440);
        let smallHeight = Math.min(metadata.height, 500);
        let {big, small} = getPaths(filePath);
        console.log(1);
        await resize({input: filePath, output: big, height});
        console.log(3);
        await resize({input: filePath, output: small, height: smallHeight});
        console.log("put in db", filePath, metadata, labels, `+${bigPic} en ${smallPic}`);
    } else if (type === 'video') {
        let metadata = await probeVideo(filePath);
        let height = Math.min(metadata.height, 1080);
        let smallHeight = Math.min(metadata.height, 500);
        let {webm, poster, smallPoster} = getPaths(filePath);
        await transcode({input: filePath, output: webm, height});
        await videoScreenshot({input: webm, output: poster, height});
        await resize({input: poster, output: smallPoster, height: smallHeight});
        console.log("put in db", filePath, metadata, `+${webm}, ${vidPoster} en ${smallVidPoster}`);
    }
    // If some failure happens, retry after timeout, then post to telegram
    return true;
}

async function removeMedia(filePath) {
    let type = getFileType(filePath);
    try {
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
    let fileName = path.basename(filePath);
    let big = path.join(bigPic, fileName + '.webp');
    let small = path.join(smallPic, fileName + '.webp');
    let webm = path.join(streamVid, fileName + '.webm');
    let poster = path.join(vidPoster, fileName + '.webp');
    let smallPoster = path.join(smallVidPoster, fileName + '.webp');
    return {big, small, webm, poster, smallPoster}
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
