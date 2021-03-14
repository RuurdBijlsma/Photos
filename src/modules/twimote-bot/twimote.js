import emotes from './emotes.js';
import https from "https";
import fs from "fs";
import c from "canvas";
import path from 'path';
import gifyParse from "gify-parse";
import ffmpeg from 'fluent-ffmpeg';
import filenamify from 'filenamify';

const telegramBackground = '#0e1621';
const telegramStickerMaxWidth = 600;
const telegramMinHeight = 100;
const telegramMaxAspectRatio = 430 / 100;

const emoteHeight = 50;
const mediaHeight = Math.max(emoteHeight, telegramMinHeight);
const fontSize = emoteHeight * 0.6;
const maxGifDuration = 20000; // Milliseconds
const horizontalPad = Math.round(emoteHeight / 5);

export async function text2media(text) {
    text = filenamify(text);
    let fileName = path.resolve(path.join('res', 'twimote', text)) + '.' + getFileType(text);

    if (await checkFileExists(fileName))
        return fileName;

    let segments = await getSegments(text);
    if (segments.some(s => s.type === 'gif')) {
        await segments2gif(segments, fileName);
    } else {
        await segments2png(segments, fileName);
    }
    return fileName;
}

export function getFileType(text) {
    for (let word of text.split(' ')) {
        if (emotes.hasOwnProperty(word) && emotes[word].animated)
            return 'mp4';
    }
    let width = getTextWidth(text);
    let isSticker = width <= telegramStickerMaxWidth;
    return isSticker ? 'webp' : 'png';
}

async function segments2png(segments, fileName) {
    let imageSegments = segments.filter(s => s.type === 'image');

    let totalWidth = segments.map(s => s.width).reduce((a, b) => a + b) + (segments.length - 1) * horizontalPad;
    let isSticker = totalWidth <= telegramStickerMaxWidth;
    let color = isSticker ? 'transparent' : telegramBackground;
    let height = mediaHeight;
    if (isSticker) {
        height = emoteHeight;
    } else {
        let aspectRatio = totalWidth / mediaHeight;
        if (aspectRatio > telegramMaxAspectRatio)
            height = Math.ceil(totalWidth / telegramMaxAspectRatio);
    }
    const yOffset = Math.round((height - emoteHeight) / 2);

    let images = await Promise.all(imageSegments.map(s => c.loadImage(s.value)));
    let canvas = c.createCanvas(totalWidth, height);
    let context = canvas.getContext('2d');
    context.fillStyle = color;
    context.fillRect(0, 0, canvas.width, canvas.height);

    for (let segment of segments) {
        let beforeSegments = segments.slice(0, segments.indexOf(segment));
        let leftPixels = beforeSegments.map(s => s.width).reduce((a, b) => a + b, 0) + horizontalPad * beforeSegments.length;
        if (segment.type === 'text') {
            context.font = `${fontSize}px Arial`;
            context.fillStyle = 'white';
            context.fillText(segment.value, leftPixels, yOffset + emoteHeight - (emoteHeight - fontSize) / 1.5);
        } else {
            let imageIndex = imageSegments.indexOf(segment);
            context.drawImage(images[imageIndex], leftPixels, yOffset, segment.width, emoteHeight);
        }
    }

    const stream = canvas.createPNGStream()
    return new Promise((resolve, reject) => {
        if (totalWidth <= telegramStickerMaxWidth) {
            ffmpeg(stream)
                .outputFormat('webp')
                .saveToFile(fileName)
                .on('error', (err, stdout, stderr) => {
                    console.log("ffmpeg webp error", err, stdout, stderr);
                    reject(err);
                })
                .on('end', () => {
                    console.log("ffmpeg webp end");
                    resolve({type: 'sticker'});
                });
        } else {
            const out = fs.createWriteStream(fileName)
            stream.pipe(out)
            out.on('finish', () => {
                resolve({type: 'image'})
            });
        }
    })
}

async function segments2gif(segments, outputPath) {
    let gifSegments = segments.filter(s => s.type === 'gif');
    let maxDuration = gifSegments.length === 0 ? 5 : Math.min(maxGifDuration, Math.max(...gifSegments.map(s => s.duration))) / 1000;

    let totalWidth = segments.map(s => s.width).reduce((a, b) => a + b) + (segments.length - 1) * horizontalPad;
    let height = mediaHeight;
    let aspectRatio = totalWidth / mediaHeight;
    if (aspectRatio > telegramMaxAspectRatio)
        height = Math.ceil(totalWidth / telegramMaxAspectRatio);
    const yOffset = Math.round((height - emoteHeight) / 2);

    console.log('max duration', maxDuration);

    let command = ffmpeg();
    let inputSegments = segments.filter(s => s.type !== 'text');
    for (let segment of inputSegments)
        command.addInput(segment.value);

    let filters = [];
    filters.push({
        filter: 'color',
        options: {
            color: telegramBackground,
            duration: maxDuration,
            size: `${totalWidth}x${height}`,
        },
        outputs: 'out',
    });

    for (let segment of segments) {
        let beforeSegments = segments.slice(0, segments.indexOf(segment));
        let leftPixels = beforeSegments.map(s => s.width).reduce((a, b) => a + b, 0) + horizontalPad * beforeSegments.length;
        if (segment.type === 'text') {
            filters.push({
                inputs: 'out',
                filter: 'drawtext',
                options: {
                    fontcolor: 'white',
                    font: 'Arial',
                    text: segment.value,
                    fontsize: fontSize,
                    x: leftPixels,
                    y: yOffset + emoteHeight / 10 + (emoteHeight - fontSize) / 2,
                },
                outputs: 'out',
            });
        } else {
            let videoIndex = inputSegments.indexOf(segment);
            filters.push({
                inputs: [`[${videoIndex}:v]`],
                filter: 'scale',
                options: `${segment.width}x${emoteHeight}`,
                outputs: 'scaled',
            });
            if (segment.type === 'gif')
                filters.push({
                    inputs: ['scaled'],
                    filter: 'loop',
                    options: {loop: -1, size: segment.frames},
                    outputs: 'scaled',
                });
            filters.push({
                inputs: ['out', 'scaled'],
                filter: 'overlay',
                options: `${leftPixels}:${yOffset}`,
                outputs: 'out',
            });
        }
    }
    filters.push({
        inputs: 'out',
        filter: 'trim',
        options: {start: 0, end: maxDuration},
        outputs: 'out',
    });

    return new Promise((resolve, reject) => {
        let gifOutput = path.resolve('test.mp4');
        command = command.complexFilter(filters, ['out'])
            .outputFormat('mp4')
            .on('start', commandLine => {
                console.log("Spawned ffmepg with command", commandLine)
            })
            .on('stderr', line => console.log('output', line))
            .on('progress', progress => console.log("Progress", progress))
            .on('error', (err, stdout, stderr) => {
                console.log("ffmpeg error", err, stdout, stderr);
                reject(err);
            })
            .on('end', () => {
                console.log("ffmpeg end");
                resolve({type: 'gif', filePath: gifOutput});
            });
        console.log(segments);
        command.saveToFile(outputPath);
    });
}

function getTextWidth(text) {
    let words = text.split(' ');
    let widths = [];
    let segmentWords = [];
    for (let word of words) {
        if (emotes.hasOwnProperty(word)) {
            if (segmentWords.length > 0) widths.push(...getTextSegments(segmentWords).map(s => s.width));
            widths.push(emotes[word].ratio * emoteHeight);
            segmentWords = [];
        } else
            segmentWords.push(word);
    }
    if (segmentWords.length > 0) widths.push(getTextSegments(segmentWords).map(s => s.width));
    return widths.reduce((a, b) => a + b, 0) + (widths.length - 1) * horizontalPad;
}

async function getSegments(text) {
    let words = text.split(' ');
    let segments = [];
    let segmentWords = [];
    for (let word of words) {
        if (emotes.hasOwnProperty(word)) {
            if (segmentWords.length > 0) segments.push(...getTextSegments(segmentWords));
            segments.push(await getEmoteSegment(word));
            segmentWords = [];
        } else
            segmentWords.push(word);
    }
    if (segmentWords.length > 0) segments.push(...getTextSegments(segmentWords));
    return segments;
}

async function getEmoteSegment(emoteName) {
    let emote = emotes[emoteName];
    let emotePath = await getEmote(emoteName, emote);
    let segment = {
        width: Math.round(emoteHeight * emote.ratio),
        type: emote.animated ? 'gif' : 'image',
        value: emotePath,
    };
    if (emote.animated) {
        const buffer = fs.readFileSync(emotePath);
        const gifInfo = gifyParse.getInfo(buffer);
        segment.duration = gifInfo.duration;
        segment.frames = gifInfo.images.length;
    }
    return segment;
}

function getTextSegments(words) {
    let text = words.join(' ');
    if (text.startsWith(' ')) {
        let nSpaces = 0;
        for (let i = 0; i < text.length; i++)
            if (text[i] === ' ') nSpaces++;
            else break;
        return [getTextSegment(text.substr(0, nSpaces)), getTextSegment(text.substr(nSpaces))];
    }
    return [getTextSegment(text)];
}

function getTextSegment(text) {
    const canvas = c.createCanvas(1, 1)
    const context = canvas.getContext('2d')
    context.font = `${fontSize}px Arial`;
    let {width} = context.measureText(text);
    return {
        width,
        type: 'text',
        value: text,
    };
}

async function getEmote(name, emote) {
    let fileName = path.resolve(path.join('res', 'twimote', name + (emote.animated ? '.gif' : '.png')));
    if (await checkFileExists(fileName))
        return fileName;

    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(fileName);
        https.get(emote.url, response => {
            response.pipe(file);
            file.on('finish', () => {
                file.close();
                resolve(fileName);
            });
        }).on('error', err => { // Handle errors
            fs.unlink(fileName, () => 0);
            reject(err.message);
        });
    });
}

async function checkFileExists(file) {
    return fs.promises.access(file, fs.constants.F_OK)
        .then(() => true)
        .catch(() => false)
}

// await text2media("widepeepoHappy R I OMEGALUL L U widepeepoHappy")