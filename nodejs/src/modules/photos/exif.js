import exif from "exif";
import parseDMS from "parse-dms";
import ffmpeg from './promise-ffmpeg.js'
import fs from "fs";
import geocode from "./reverse-geocode.js";
import path from "path";
import probeSize from 'probe-image-size';
import {filenameToDate} from "exif-date-fix";
import {format} from 'date-fns'
import modifyExif from 'modify-exif'
import {temp} from "./watchAndSynchonize.js";
import {exec} from "child_process";
import Clog from '../../Clog.js'
import getExifModule from "get-exif";

const console = new Clog('exif');

const {ExifImage} = exif;

export async function transferExif(sourceFile, destinationFile) {
    console.log(`Transferring exif from ${sourceFile} to ${destinationFile}`);
    const source = await fs.promises.readFile(sourceFile);
    let sourceExif = getExifModule(source);

    const copyKeys = ['Exif', 'GPS', '0th', '1st', 'Interop'];
    const dimensionKeys = ['40962', '40963'];

    const newFile = modifyExif(await fs.promises.readFile(destinationFile), data => {
        // console.log('sourceExif', sourceExif);
        for (let copyKey of copyKeys)
            for (let key in sourceExif[copyKey]) {
                if (sourceExif[copyKey].hasOwnProperty(key) && !dimensionKeys.includes(key)) {
                    data[copyKey][key] = sourceExif[copyKey][key];
                }
            }
    });

    // let newExif = getExifModule(newFile);
    // console.log(newExif);

    // let writtenExif = getExifModule(newFile).Exif;
    await fs.promises.writeFile(destinationFile, newFile);
    // console.log('written', destinationFile,);
}

// await rotateImage('20170723_175323.jpg', 0.04, '20170723_175323_rotated.jpg');

export async function dateFromFile(filePath) {
    let date = filenameToDate(path.basename(filePath));
    if (date !== null) return date;
    let fileStat = await fs.promises.stat(filePath);
    return dateToString(new Date(fileStat.birthtimeMs));
}

export function dateToString(d) {
    return format(d, 'yyyy-MM-dd HH:mm:ss');
}

export async function getCreateDate(image, exifData = null) {
    let createDate = null;
    if (exifData !== null) {
        let timeFields = ['DateTimeOriginal', 'CreateDate'];
        for (let timeField of timeFields) {
            if (exifData.exif[timeField] &&
                exifData.exif[timeField].includes(' ') &&
                exifData.exif[timeField].match(/[^ ]+ [^ ]+/)) {
                let [date, time] = exifData.exif[timeField].split(' ');
                date = date.replace(/:/g, '/');
                createDate = `${date} ${time}`;
            }
        }
    }

    if (createDate === null) {
        createDate = await dateFromFile(image);
    }

    return createDate;
}

/**
 * Get image dimensions
 * @param filePath
 * @param exifData
 * @returns {Promise<{width, height}|null>}
 */
export async function imageSize(filePath, exifData = null) {
    let imgSize;

    let orientation = exifData?.image?.Orientation ?? 1;
    let swap = orientation >= 5;

    try {
        imgSize = await probeSize(fs.createReadStream(filePath));
        // console.log("Using probeSize");
        // noinspection JSSuspiciousNameCombination
        return swap ? {width: imgSize.height, height: imgSize.width} : {width: imgSize.width, height: imgSize.height};
    } catch (e) {
    }

    try {
        let result = await ffmpeg.ffprobe(filePath);
        // console.log("Using ffprobe");
        let {width, height} = result.streams[0];
        // noinspection JSSuspiciousNameCombination
        return swap ? {width: height, height: width} : {width, height};
    } catch (e) {
    }

    let width = exifData?.image?.ImageWidth
    let height = exifData?.image?.ImageHeight;
    if (Number.isFinite(width) && Number.isFinite(height)) {
        // console.log("Using exif");
        // noinspection JSSuspiciousNameCombination
        return swap ? {width: height, height: width} : {width, height};
    }

    return null;
}

export async function loadExif(image) {
    return new Promise((resolve, reject) => {
        new ExifImage({image}, async (error, data) => {
            if (error)
                return reject(error);
            resolve(data);
        });
    });
}

export async function getExif(image) {
    let data;
    try {
        data = await loadExif(image);
    } catch (e) {
        let {size} = await fs.promises.stat(image);
        let createDate = await dateFromFile(image);
        let imgDim = await imageSize(image);
        if (imgDim === null)
            throw new Error(`Can't get image dimensions for ${image}`);

        console.log(`No exif for ${image}`);
        return {
            type: 'image', subType: image.endsWith('gif') ? 'animation' : 'none', ...imgDim, duration: null,
            size, createDate, gps: null, exif: {}
        };
    }

    let gps = null;
    if (data.gps.GPSLatitude && data.gps.GPSLongitude) {
        let lad = data.gps.GPSLatitude;
        let latString = `${lad[0]}??${lad.slice(1).join(`'`)}"${data.gps.GPSLatitudeRef}`;
        let lod = data.gps.GPSLongitude;
        let lonString = `${lod[0]}??${lod.slice(1).join(`'`)}"${data.gps.GPSLongitudeRef}`;
        let {lat, lon} = parseDMS(`${latString} ${lonString}`);
        gps = {latitude: lat, longitude: lon};
        gps.altitude = data.gps.GPSAltitude;
        let geocodeData = await geocode(gps);
        gps = {...gps, ...geocodeData};
    }

    let fileStat = await fs.promises.stat(image);
    let imgDim = await imageSize(image, data);
    if (imgDim === null)
        throw new Error(`Can't get image dimensions for ${image}`);

    let createDate = await getCreateDate(image, data);

    let exifData = {
        Make: data.image.Make,
        Model: data.image.Model,
        Orientation: data.image.Orientation,
        XResolution: data.image.XResolution,
        YResolution: data.image.YResolution,
        ResolutionUnit: data.image.ResolutionUnit,
        ...data.exif,
    }
    for (let field in exifData) {
        let value = exifData[field];
        if (value === undefined)
            delete exifData[field];
        if (value instanceof Buffer)
            delete exifData[field];
        if (typeof value === "string" && value.includes('\x00'))
            delete exifData[field];
    }

    let filename = path.basename(image);
    let subType = 'none';
    if (filename.includes("PORTRAIT") && filename.includes("COVER"))
        subType = 'Portrait';
    else if (filename.startsWith('PANO'))
        subType = 'VR';

    return {
        type: 'image', subType, ...imgDim, duration: null,
        size: fileStat.size, createDate, gps, exif: exifData
    };
}

export async function probeVideo(videoPath) {
    let {streams, format} = await ffmpeg.ffprobe(videoPath);
    let video = streams.find(s => s.codec_type === 'video');
    let audio = streams.find(s => s.codec_type === 'audio');
    let rotation = +video.rotation;
    let width, height;
    if (rotation % 90 === 0 && rotation % 180 !== 0) {
        // noinspection JSSuspiciousNameCombination
        width = video.height;
        // noinspection JSSuspiciousNameCombination
        height = video.width;
    } else {
        width = video.width;
        height = video.height;
    }
    let duration = Math.round(1000 * format.duration);
    let createDate = null;
    if (format.tags.creation_time) {
        // timezone is included in tags.creation_time so `new Date()` is allowed
        createDate = dateToString(new Date(format.tags.creation_time));
    }
    if (!createDate)
        createDate = await dateFromFile(videoPath);
    let gps = null;
    if (format.tags.location !== undefined) {
        let [[latitude], [longitude]] = format.tags.location.matchAll(/[+-]\d+\.\d+/g)
        latitude = +latitude;
        longitude = +longitude;
        gps = {latitude, longitude, altitude: null};
        let geocodeData = await geocode(gps);
        gps = {...gps, ...geocodeData};
    }
    let size = format.size;
    let exifData = {
        ...format.tags,
        video,
        audio: audio ?? null,
        ...format,
    };
    if (audio) {
        delete exifData.audio.disposition;
        delete exifData.audio.tags;
    }
    delete exifData.video.disposition;
    delete exifData.video.tags;
    delete exifData.filename;
    delete exifData.tags;

    let slowMotion = false;
    if (exifData.video.avg_frame_rate &&
        exifData.video.avg_frame_rate.includes('/') &&
        exifData.hasOwnProperty('com.android.capture.fps')) {
        let captureFps = +exifData['com.android.capture.fps'];
        let [fps1, fps2] = exifData.video.avg_frame_rate.split('/').map(n => +n);
        // Capture fps is sometimes a bit higher than actual fps on non slomo videos
        slowMotion = captureFps / 1.9 > fps1 / fps2;
    }
    let subType = slowMotion ? 'slomo' : 'none';
    return {
        type: 'video', subType, width, height,
        duration, size, createDate, gps, exif: exifData
    };
}

export async function updateVideoDate(filePath, date) {
    let tempOutput = path.resolve(path.join(temp, path.basename(filePath)));
    const newDateString = dateToString(date);

    let success = await new Promise((resolve, reject) => {
        let command = `ffmpeg -y -i "${path.resolve(filePath)}" -c copy -metadata creation_time="${newDateString}" "${tempOutput}"`;
        console.log(command);
        exec(command, (error, stderr, stdout) => {
            if (error) {
                console.warn('video date change error', error);
                return resolve(false);
            }
        }).on('close', () => resolve(true));
    });
    if (success) {
        await fs.promises.rename(tempOutput, filePath);
    }
}

export async function updatePhotoDate(filePath, date) {
    const originalFile = await fs.promises.readFile(filePath);
    const newDateString = format(date, 'yyyy:MM:dd HH:mm:ss');

    const newFile = modifyExif(originalFile, data => {
        // 36867: tag ID of DateTimeOriginal tag
        data.Exif['36867'] = newDateString;
    });

    await fs.promises.writeFile(filePath, newFile);
}


// probeVideo(path.resolve('PXL_20210831_184546467.mp4')).then(c => {
//     console.log(c);
// })

// getExif(path.resolve('IMG_20140709_170611740.jpg')).then(d => {
//     console.log(d);
// });

// updateVideoDate(path.join(config.media, 'VID_20210514_033314.mp4'), new Date('1 dec 2023')).then(r => {
//     console.log(r);
// });

// probeVideo('./photos/home.mp4');
