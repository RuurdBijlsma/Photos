import exif from "exif";
import parseDMS from "parse-dms";
import ffmpeg from './promise-ffmpeg.js'
import fs from "fs";
import geocode from "./reverse-geocode.js";
import path from "path";
import probeSize from 'probe-image-size';

const {ExifImage} = exif;

export async function probeVideo(videoPath) {
    let {streams, format} = await ffmpeg.ffprobe(videoPath);
    let video = streams.find(s => s.codec_type === 'video');
    let audio = streams.find(s => s.codec_type === 'audio');
    let width = video.width;
    let height = video.height;
    let duration = Math.round(1000 * format.duration);
    let createDate = null;
    if (format.tags.creation_time)
        createDate = new Date(format.tags.creation_time);
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
        slowMotion = captureFps > (fps1 / fps2);
    }
    let subType = slowMotion ? 'slomo' : 'none';
    return {
        type: 'video', subType, width, height,
        duration, size, createDate, gps, exif: exifData
    };
}

export async function getExif(image) {
    return new Promise((resolve, reject) => {
        new ExifImage({image}, async (error, data) => {
            if (error) {
                let imgSize;
                try {
                    imgSize = await probeSize(fs.createReadStream(image));
                } catch (e) {
                    return reject(`img resize error on img with no exif: ${image}, \n ${JSON.stringify(e)}`);
                }
                let fileStat = await fs.promises.stat(image);
                let width = imgSize.width;
                let height = imgSize.height;
                console.log(`No exif for ${image}`)
                return resolve({
                    type: 'image', subType: 'none', width, height, duration: null,
                    size: fileStat.size, createDate: new Date(fileStat.birthtimeMs), gps: null, exif: {}
                });
            }

            let gps = null;
            if (data.gps.GPSLatitude && data.gps.GPSLongitude) {
                let lad = data.gps.GPSLatitude;
                let latString = `${lad[0]}°${lad.slice(1).join(`'`)}"${data.gps.GPSLatitudeRef}`;
                let lod = data.gps.GPSLongitude;
                let lonString = `${lod[0]}°${lod.slice(1).join(`'`)}"${data.gps.GPSLongitudeRef}`;
                let {lat, lon} = parseDMS(`${latString} ${lonString}`);
                gps = {latitude: lat, longitude: lon};
                gps.altitude = data.gps.GPSAltitude;
                let geocodeData = await geocode(gps);
                gps = {...gps, ...geocodeData};
            }

            let {size} = await fs.promises.stat(image);
            let width = data.image.ImageWidth
            let height = data.image.ImageHeight;
            if (!isFinite(width) || !isFinite(height)) {
                try {
                    let imgSize = await probeSize(fs.createReadStream(image));
                    width = imgSize.width;
                    height = imgSize.height;
                } catch (e) {
                    return reject(`Couldn't get image size for ${
                        image
                    }`, e);
                }
            }


            let createDate = null;
            if (data.exif.CreateDate && data.exif.CreateDate.includes(' ')) {
                let [date, time] = data.exif.CreateDate.split(' ');
                date = date.replace(/:/gi, '/');
                createDate = new Date(`${date}, ${time}`);
            }

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
            if (filename.includes("PORTRAIT" && filename.includes("COVER")))
                subType = 'Portrait';
            else if (filename.startsWith('PANO'))
                subType = 'VR';
            resolve({
                type: 'image', subType, width, height, duration: null,
                size, createDate, gps, exif: exifData
            });
        });
    });
}


// probeVideo('./photos/home.mp4');
// getExif('./photos/20150729_211225.jpg')
