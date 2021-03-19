import https from "https";
import fs from "fs";
import c from "canvas";
import path from 'path';
import gifyParse from "gify-parse";
import ffmpeg from 'fluent-ffmpeg';
import filenamify from 'filenamify';
import {Emote} from "../../database/models/EmoteModel.js";
import seq from "sequelize";

const {Op} = seq;

const photoBackground = '#0e1621';
const videoBackground = '#182533';
const telegramMinHeight = 100;
const photoMaxAspectRatio = 430 / 100;
const videoMaxAspectRatio = 310 / 100;
const minAspectRatio = 1;
const emoteHeight = 90;

const telegramStickerMaxWidth = emoteHeight * 10;
const mediaHeight = Math.max(emoteHeight, telegramMinHeight);
const fontSize = emoteHeight * 0.6;
const maxGifDuration = 20000; // Milliseconds
const horizontalPad = Math.round(emoteHeight / 5);

export async function text2media(text) {
    let textFile = filenamify(text);
    let fileName = path.resolve(path.join('res', 'twimote', 'cache', textFile)) + '.' + await getFileType(text);

    if (await checkFileExists(fileName))
        return fileName;

    // if (process.platform !== 'win32' && fileName.endsWith('mp4'))
    //     return await text2media('YEP animated emotes not supported yet');

    let segments = await getSegments(text);
    if (segments.some(s => s.type === 'gif')) {
        await segments2mp4(segments, fileName);
    } else {
        await segments2image(segments, fileName);
    }
    return fileName;
}

export async function getFileType(text) {
    for (let word of text.split(' ')) {
        let emote = await Emote.findOne({where: {name: {[Op.iLike]: `${word}`}}});
        if (emote !== null && emote.animated)
            return 'mp4';
    }
    let {width} = await getTextSize(text);
    let isSticker = width <= telegramStickerMaxWidth;
    return isSticker ? 'webp' : 'png';
}

function getImageHeight(width) {
    let isSticker = width <= telegramStickerMaxWidth;
    let height = mediaHeight;
    if (isSticker) {
        height = emoteHeight;
    } else {
        let aspectRatio = width / mediaHeight;
        if (aspectRatio > photoMaxAspectRatio)
            height = Math.ceil(width / photoMaxAspectRatio);
        else if (aspectRatio < minAspectRatio)
            height = Math.floor(width / minAspectRatio);
        console.log("PNG height", height);
    }
    return height;
}

function fit(contains) {
    return (parentWidth, parentHeight, childWidth, childHeight, scale = 1, offsetX = 0.5, offsetY = 0.5) => {
        const childRatio = childWidth / childHeight
        const parentRatio = parentWidth / parentHeight
        let width = parentWidth * scale
        let height = parentHeight * scale

        if (contains ? (childRatio > parentRatio) : (childRatio < parentRatio)) {
            height = width / childRatio
        } else {
            width = height * childRatio
        }

        return {
            width,
            height,
            offsetX: (parentWidth - width) * offsetX,
            offsetY: (parentHeight - height) * offsetY
        }
    }
}

const contain = fit(true);

async function segments2image(segments, fileName) {
    let imageSegments = segments.filter(s => s.type === 'image');

    let totalWidth = segments.map(s => s.width).reduce((a, b) => a + b) + (segments.length - 1) * horizontalPad;
    let isSticker = totalWidth <= telegramStickerMaxWidth;
    let color = isSticker ? 'transparent' : photoBackground;
    let height = getImageHeight(totalWidth);
    const yOffset = Math.round((height - emoteHeight) / 2);

    let images = await Promise.all(imageSegments.map(s => c.loadImage(s.value)));
    let canvas = c.createCanvas(totalWidth, height);
    let context = canvas.getContext('2d');
    context.clearRect(0, 0, canvas.width, canvas.height)
    context.fillStyle = color;
    context.fillRect(0, 0, canvas.width, canvas.height);

    for (let segment of segments) {
        let beforeSegments = segments.slice(0, segments.indexOf(segment));
        let leftPixels = beforeSegments.map(s => s.width).reduce((a, b) => a + b, 0) + horizontalPad * beforeSegments.length;
        if (segment.type === 'text') {
            context.font = `${fontSize}px Arial`;
            context.fillStyle = 'white';
            context.fillText(
                segment.value,
                leftPixels,
                (yOffset + emoteHeight - (emoteHeight - fontSize) / 1.5)
            );
        } else {
            let imageIndex = imageSegments.indexOf(segment);
            context.drawImage(
                images[imageIndex],
                leftPixels,
                yOffset,
                segment.width,
                emoteHeight,
            );
        }
    }

    return new Promise((resolve, reject) => {
        if (isSticker) {
            let stickerCanvas;
            if (canvas.width <= 512 && canvas.height <= 512) {
                stickerCanvas = c.createCanvas(512, canvas.height);
                let stickerContext = stickerCanvas.getContext('2d');
                stickerContext.drawImage(canvas, 512 / 2 - canvas.width / 2, 0);
            } else {
                const {width: stickerWidth, height: stickerHeight} = contain(512, 512, totalWidth, height);
                stickerCanvas = c.createCanvas(stickerWidth, stickerHeight);
                let stickerContext = stickerCanvas.getContext('2d');
                stickerContext.drawImage(canvas, 0, 0, stickerWidth, stickerHeight);
            }
            ffmpeg(stickerCanvas.createPNGStream())
                .format('webp')
                .saveToFile(fileName)
                .on('error', (err, stdout, stderr) => {
                    console.log("ffmpeg error", err, stdout, stderr);
                    reject(err);
                })
                .on('end', () => {
                    console.log("webp ffmpeg end");
                    resolve();
                });

        } else {
            const stream = canvas.createPNGStream();
            const out = fs.createWriteStream(fileName);
            stream.pipe(out)
            out.on('finish', () => {
                resolve()
            });
        }
    })
}

function getGifHeight(width) {
    let height = mediaHeight;
    let aspectRatio = width / mediaHeight;
    if (aspectRatio > videoMaxAspectRatio)
        height = Math.ceil(width / videoMaxAspectRatio);
    else if (aspectRatio < minAspectRatio)
        height = Math.floor(width / minAspectRatio);
    return Math.max(telegramMinHeight, height);
}

async function segments2mp4(segments, outputPath) {
    let gifSegments = segments.filter(s => s.type === 'gif');
    let maxDuration = gifSegments.length === 0 ? 5 : Math.min(maxGifDuration, Math.max(...gifSegments.map(s => s.duration))) / 1000;

    let totalWidth = segments.map(s => s.width).reduce((a, b) => a + b) + (segments.length - 1) * horizontalPad;
    let xOffset = Math.max((telegramMinHeight - totalWidth) / 2, 0);
    totalWidth = Math.max(telegramMinHeight, totalWidth);
    let height = getGifHeight(totalWidth);
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
            color: videoBackground,
            duration: maxDuration,
            size: `${totalWidth}x${height}`,
        },
        outputs: 'out',
    });

    for (let segment of segments) {
        let beforeSegments = segments.slice(0, segments.indexOf(segment));
        let leftPixels = xOffset + beforeSegments.map(s => s.width).reduce((a, b) => a + b, 0) + horizontalPad * beforeSegments.length;
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
                resolve({type: 'gif'});
            });
        console.log(segments);
        command.saveToFile(outputPath);
    });
}

export async function getTextSize(text) {
    let words = text.split(' ');
    let widths = [];
    let durations = [];
    let segmentWords = [];
    let animated = false;
    for (let word of words) {
        let emote = await Emote.findOne({where: {name: {[Op.iLike]: `${word}`}}});
        if (emote !== null) {
            if (segmentWords.length > 0) widths.push(...getTextSegments(segmentWords).map(s => s.width));
            widths.push(emote.ratio * emoteHeight);
            if (emote.animated) {
                animated = true;
                durations.push(emote.duration);
            }
            segmentWords = [];
        } else
            segmentWords.push(word);
    }
    if (segmentWords.length > 0) widths.push(...getTextSegments(segmentWords).map(s => s.width));
    let width = Math.round(widths.reduce((a, b) => a + b, 0) + (widths.length - 1) * horizontalPad);
    let height = animated ? Math.round(getGifHeight(width)) : Math.round(getImageHeight(width));
    let duration = Math.min(maxGifDuration, Math.max(...durations, 0));
    return {width, height, duration, animated};
}

async function getSegments(text) {
    let words = text.split(' ');
    let segments = [];
    let segmentWords = [];
    for (let word of words) {
        let emote = await Emote.findOne({where: {name: {[Op.iLike]: `${word}`}}});
        if (emote !== null) {
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
    let emote = await Emote.findOne({where: {name: {[Op.iLike]: `${emoteName}`}}});
    let emotePath = await getEmote(emoteName, emote);
    let segment = {
        width: Math.round(emoteHeight * emote.ratio),
        type: emote.animated ? 'gif' : 'image',
        value: emotePath,
    };
    if (emote.animated) {
        const buffer = await fs.promises.readFile(emotePath);
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
    let fileName = path.resolve(path.join('res', 'twimote', 'emotes', filenamify(name) + (emote.animated ? '.gif' : '.png')));
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

export async function getSuggestions(text) {
    let words = text.split(' ');
    let lastWord = words[words.length - 1];
    let nextEmotes = await Emote.findAll({
        where: {name: {[Op.iLike]: `${lastWord}%`}},
        limit: lastWord.length > 3 ? 3 : 1,
    });
    let sentenceStart = words.slice(0, words.length - 1).join(' ');
    let suggestions = nextEmotes
        .filter(emote => emote.name !== lastWord)
        .map(emote => `${sentenceStart} ${emote.name}`);
    return [text, ...suggestions];
}

export async function search(query) {
    return await Emote.findAll({
        where: {name: {[Op.iLike]: `%${query}%`},},
        limit: 20,
    });
}

async function checkFileExists(file) {
    return fs.promises.access(file, fs.constants.F_OK)
        .then(() => true)
        .catch(() => false)
}

// await text2media("widepeepoHappy R I OMEGALUL L U widepeepoHappy")