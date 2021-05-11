import ffmpeg from "./promise-ffmpeg.js";
import sharp from "sharp";

export async function resize({input, output, width = null, height = null}) {
    console.log(2);
    return await sharp(input)
        .rotate()
        .resize(width, height)
        .webp({
            quality: 85,
            smartSubsample: true,
        })
        .toFile(output);
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
        let command = ffmpeg(input, {
            niceness: -10,
        })
            .videoBitrate(bitrate)
            .size(`${width ?? '?'}x${height ?? '?'}`)
            .videoCodec('libvpx-vp9')
            .format('webm')
            .on('start', commandLine => {
                console.log('Spawned Ffmpeg with command: ' + commandLine);
            })
            .on('progress', progress => {
                console.log(`Processing: ${progress.timemark} / ${duration}`);
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
                console.log("Ffmpeg done");
                resolve();
            })
            .saveToFile(output);

        signal?.addEventListener?.('abort', () => command.kill());
    })
}
