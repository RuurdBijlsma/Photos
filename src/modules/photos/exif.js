import exif from "exif";
import parseDMS from "parse-dms";
import ffmpeg from './promise-ffmpeg.js'
import fs from "fs";
import geocode from "./reverse-geocode.js";
import {StringDecoder} from "string_decoder";
import path from "path";
import {promisify} from 'util'
import sizeOfSync from "image-size";

const sizeOf = promisify(sizeOfSync);

const decoder = new StringDecoder('latin1');

const {ExifImage} = exif;

export async function probeVideo(videoPath) {
    let {streams, format} = await ffmpeg.ffprobe(videoPath);
    let video = streams.find(s => s.codec_type === 'video');
    let audio = streams.find(s => s.codec_type === 'audio');
    let width = video.width;
    let height = video.height;
    let duration = format.duration;
    let createDate = new Date(format.tags.creation_time);
    let gps = null;
    if (format.tags.location !== undefined) {
        let [[lat], [lon]] = format.tags.location.matchAll(/[+-]\d+\.\d+/g)
        lat = +lat;
        lon = +lon;
        gps = {lat, lon, altitude: null};
        let geocodeData = await geocode({
            latitude: gps.lat,
            longitude: gps.lon
        });
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

    let slowmotion = false;
    if (exifData.video.avg_frame_rate &&
        exifData.video.avg_frame_rate.includes('/') &&
        exifData.hasOwnProperty('com.android.capture.fps')) {
        let captureFps = +exifData['com.android.capture.fps'];
        let [fps1, fps2] = exifData.video.avg_frame_rate.split('/').map(n => +n);
        slowmotion = captureFps > (fps1 / fps2);
    }
    let subType = slowmotion ? 'slomo' : 'none';
    return {type: 'video', subType, width, height, duration, size, createDate, gps, exif: exifData};
}

// probeVideo('./photos/home.mp4');
// getExif('./photos/20150804_192803.jpg')

export async function getExif(image) {
    return new Promise((resolve, reject) => {
        new ExifImage({image}, async (error, data) => {
            if (error)
                return reject(error);

            let gps = null;
            if (data.gps.GPSLatitude && data.gps.GPSLongitude) {
                let lad = data.gps.GPSLatitude;
                let latString = `${lad[0]}°${lad.slice(1).join(`'`)}"${data.gps.GPSLatitudeRef}`;
                let lod = data.gps.GPSLongitude;
                let lonString = `${lod[0]}°${lod.slice(1).join(`'`)}"${data.gps.GPSLongitudeRef}`;
                gps = parseDMS(`${latString} ${lonString}`);
                gps.altitude = data.gps.GPSAltitude;
                let geocodeData = await geocode({
                    latitude: gps.lat,
                    longitude: gps.lon
                });
                gps = {...gps, ...geocodeData};
            }

            let {size} = await fs.promises.stat(image);

            let {width, height} = await sizeOf(image);

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
            for (let field in exifData)
                if (exifData[field] === undefined)
                    delete exifData[field];
            if (exifData.ExifVersion)
                exifData.ExifVersion = decoder.write(exifData.ExifVersion);
            if (exifData.FlashpixVersion)
                exifData.FlashpixVersion = decoder.write(exifData.FlashpixVersion);
            if (exifData.ComponentsConfiguration)
                exifData.ComponentsConfiguration = Array.from(exifData.ComponentsConfiguration);
            if (exifData.UserComment)
                exifData.UserComment = decoder.write(exifData.UserComment);
            if (exifData.MakerNote)
                exifData.MakerNote = decoder.write(exifData.MakerNote);

            let fileName = path.basename(image);
            let subType = 'none';
            if (fileName.includes("PORTRAIT" && fileName.includes("COVER")))
                subType = 'Portrait';
            else if (fileName.startsWith('PANO'))
                subType = 'VR';
            resolve({type: 'image', subType, width, height, size, createDate, gps, exif: exifData});
        });
    });
}
