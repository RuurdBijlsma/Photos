import ffmpeg from "./promise-ffmpeg.js";
import Clog from '../../Clog.js'
import Canvas from "canvas";
import {imageSize, transferExif} from "./exif.js";
import fs from "fs";
import { ExifImage } from "exif";

const {loadImage, createCanvas} = Canvas;
const console = new Clog('transcode');

//Exif orientation
//1 = 0 degrees: the correct orientation, no adjustment is required.
//2 = 0 degrees, mirrored: image has been flipped back-to-front.
//3 = 180 degrees: image is upside down.
//4 = 180 degrees, mirrored: image has been flipped back-to-front and is upside down.
//5 = 90 degrees: image has been flipped back-to-front and is on its side.
//6 = 90 degrees, mirrored: image is on its side.
//7 = 270 degrees: image has been flipped back-to-front and is on its far side.
//8 = 270 degrees, mirrored: image is on its far side.

//FFMPEG orientation
// 0 = 90CounterClockwise and Vertical Flip (default)
// 1 = 90Clockwise
// 2 = 90CounterClockwise
// 3 = 90Clockwise and Vertical Flip

export async function resizeImage({input, orientation = 1, output, width = null, height = null}) {
    return new Promise((resolve, reject) => {
        let flip = orientation === 6;
        let size = flip ?
            width === null && height === null ? '100%' : `${height ?? '?'}x${width ?? '?'}` :
            width === null && height === null ? '100%' : `${width ?? '?'}x${height ?? '?'}`;
        let command = ffmpeg(input).size(size);
        if (orientation === 2)
            command.videoFilter([`hflip`]);
        if (orientation === 3)
            command.videoFilter([`vflip`, `hflip`]);
        if (orientation === 4)
            command.videoFilter([`vflip`]);
        if (orientation === 5)
            command.videoFilter([`transpose=${3}`]);
        if (orientation === 6)
            command.videoFilter([`transpose=${1}`]);
        if (orientation === 7)
            command.videoFilter([`transpose=${0}`]);
        if (orientation === 8)
            command.videoFilter([`transpose=${2}`]);
        command.format('webp')
            .outputOptions('-map_metadata 0')
            // .on('start', line => {
            //     console.log(line);
            // })
            .on('error', e => {
                console.warn('ffmpeg photo resize error', e);
                reject(e);
            })
            .on('end', () => {
                resolve();
            })
            .saveToFile(output);
    })
}

export async function rotateImage(image, radians, output) {
    let imgDim;
    try {
        let data = await ExifImage(image);
        imgDim = await imageSize(image, data);
    } catch (e) {
        console.warn(`Can't rotate image ${image}!`, e.message);
        return false;
    }
    if (imgDim === null)
        console.warn(`Can't rotate image ${image}! image dimensions can't be retrieved`);
    let canvas = createCanvas(imgDim.width, imgDim.height);
    let context = canvas.getContext('2d');
    let img = await loadImage(image);

    //Rotate image
    let cc = getCropCoordinates(radians, {w: img.width, h: img.height});
    canvas.width = cc.w;
    canvas.height = cc.h;

    canvasDrawImage(context, img, cc.x, cc.y, radians);
    // high quality jpeg
    let stream = canvas.createJPEGStream({
        quality: 0.95,
        chromaSubsampling: false,
    });
    // temp file is needed in case image path is equal to output path
    // We can't overwrite the original yet because there is exif data in there
    let tempOutput = output + '_temp.jpg';
    const out = fs.createWriteStream(tempOutput);
    stream.pipe(out);
    return new Promise(resolve => {
        out.on('finish', async () => {
            await transferExif(image, tempOutput);
            await fs.promises.rename(tempOutput, output);
            resolve(true);
        });
        out.on('error', e => {
            console.warn("jpeg write error", e);
            resolve(false);
        })
    });
}

function canvasDrawImage(ctx, img, x, y, angle = 0) {
    ctx.translate(ctx.canvas.width / 2, ctx.canvas.height / 2);
    ctx.rotate(angle);
    ctx.drawImage(img, -img.width / 2, -img.height / 2, img.width, img.height);
    ctx.rotate(-angle);
    ctx.translate(-ctx.canvas.width / 2, -ctx.canvas.height / 2);
}

function getCropCoordinates(angleInRadians, imageDimensions) {
    const ang = angleInRadians;
    const img = imageDimensions;

    const quadrant = Math.floor(ang / (Math.PI / 2)) & 3;
    const sign_alpha = (quadrant & 1) === 0 ? ang : Math.PI - ang;
    const alpha = (sign_alpha % Math.PI + Math.PI) % Math.PI;

    const bb = {
        w: img.w * Math.cos(alpha) + img.h * Math.sin(alpha),
        h: img.w * Math.sin(alpha) + img.h * Math.cos(alpha)
    };

    const gamma = img.w < img.h ? Math.atan2(bb.w, bb.h) : Math.atan2(bb.h, bb.w);

    const delta = Math.PI - alpha - gamma;

    const length = img.w < img.h ? img.h : img.w;
    const d = length * Math.cos(alpha);
    const a = d * Math.sin(alpha) / Math.sin(delta);

    const y = a * Math.cos(gamma);
    const x = y * Math.tan(gamma);

    return {
        x: Math.round(x),
        y: Math.round(y),
        w: Math.round(bb.w - 2 * x),
        h: Math.round(bb.h - 2 * y),
    };
}

// resize({input: './photos/IMG_20200731_203422.jpg', output: 'test.webp', height: 500})

export async function videoScreenshot({input, output, width = null, height = null, signal = null}) {
    return await ffmpeg.screenshot({file: input, output: output, width, height, signal});
}

// transcode({input: './photos/vid.mp4', output: 'test3.webm'});

// Webm stream won't work with express content-range stream setup
// Transcode mp4 to dash/hls
// ffmpeg -i vid.mp4 -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 -b:v:0 1000k  -c:v:0 libx264 -filter:v:0 "scale=-2:480"  -b:v:1 6000k -c:v:1 libx264 -filter:v:1 "scale=-2:1080" -use_timeline 1 -use_template 1 -window_size 6 -adaptation_sets "id=0,streams=v  id=1,streams=a" -hls_playlist true -f dash m3u8/output.mpd
export async function transcode({input, output, height = null, width = null, bitrate = 5555, signal = null}) {
    return new Promise((resolve, reject) => {
        if (width === null && height === null)
            return reject("Width and height can't both be null");

        let duration = '';
        let time = '';
        let command = ffmpeg(input, {
            niceness: -10,
        })
            .videoBitrate(bitrate)
            .size(`${width ?? '?'}x${height ?? '?'}`)
            .videoCodec('libvpx-vp9')
            .format('webm')
            .on('start', commandLine => {
                // console.log('Spawned Ffmpeg with command: ' + commandLine);
            })
            .on('progress', progress => {
                let newTime = progress.timemark.substring(0, 8);
                if (newTime !== time) {
                    time = newTime;
                    console.log(`Processing: ${progress.timemark} / ${duration}`);
                }
            })
            .on('stderr', line => {
                if (line.includes('Duration: ')) {
                    duration = line.split("Duration: ")[1].split(', ')[0];
                }
                // console.log(line);
            })
            .on('error', e => {
                console.warn('ffmpeg error', e);
                reject(e);
            })
            .on('end', () => {
                console.log("Ffmpeg video transcode done");
                resolve();
            })
            .saveToFile(output);

        signal?.addEventListener?.('abort', () => command.kill());
    })
}
