import ApiModule from "../../ApiModule.js";
import {text2media, getTextSize, getFileType} from './twimote.js';
import fs from "fs";
import TelegramBot from "node-telegram-bot-api";
import tokens from "../../../res/twimote/tokens.json";

// TODO
// Enable tokens for security
// Webhooks
// perma cache text to sticker file id

export default class TwimoteModule extends ApiModule {
    constructor() {
        super();
        this.tokens = {};
        if (process.platform === 'win32')
            this.botSetup();
    }

    botSetup() {
        const bot = new TelegramBot(tokens.telegram, {polling: true});
        console.log("Telegram bot is running")

        bot.on('inline_query', async ({id, from, query, offset}) => {
            if (query === '') {
                let suggestions = ['YEP'];
                let message = await bot.answerInlineQuery(id, suggestions.map(suggestion => ({
                    type: 'article',
                    id: Math.floor(Math.random() * 10000000),
                    title: suggestion,
                    input_message_content: {
                        message_text: suggestion,
                    },
                })));
                console.log('empty message', message);
                return;
            }
            // let token = await Utils.getToken();
            // this.tokens[token] = {expiryDate: (new Date) + 10000};
            // setTimeout(() => {
            //     if (this.tokens.hasOwnProperty(token))
            //         delete this.tokens[token];
            // }, 10000);

            let text = query.substr(0, 2000);
            let randomID = Math.round(Math.random() * 1000000);
            let type = getFileType(text);
            let url = `https://api.ruurd.dev/twimote?text=${encodeURIComponent(text)}&r=${randomID}&type=${type}`;
            console.log(url);
            let {width, height, duration, animated} = getTextSize(text);
            if (type === 'mp4') {
                await bot.answerInlineQuery(id, [{
                    type: 'mpeg4_gif',
                    id: randomID,
                    mpeg4_url: url,
                    mpeg4_width: width,
                    mpeg4_height: height,
                    thumb_url: url,
                    mpeg4_duration: duration,
                    thumb_mime_type: 'video/mp4',
                }]);
            } else if (type === 'jpg') {
                await bot.answerInlineQuery(id, [{
                    type: 'photo',
                    id: randomID,
                    photo_url: url,
                    photo_width: width,
                    photo_height: height,
                    thumb_url: url,
                }])
            } else if (type === 'webp') {
                let file = await text2media(text);
                let msg = await bot.sendSticker(tokens.stickerDump, file);
                await bot.answerInlineQuery(id, [{
                    type: 'sticker',
                    id: randomID,
                    sticker_file_id: msg.sticker.file_id,
                }]);
            }
        });

        bot.on('message', async (msg) => {
            // console.log("message", msg);
            if (!msg.text)
                return;

            await bot.sendMessage(msg.chat.id, "This bot should only be used inline (i.e. @twimotebot YEP)");
        });
    }

    setRoutes(app, io, db) {
        app.get('/twimote/', async (req, res) => {
            // let token = this.tokens[req.query.token];
            // if (
            //     req.query.token === undefined ||
            //     !token ||
            //     token.expiryDate === undefined ||
            //     isNaN(token.expiryDate) ||
            //     +token.expiryDate < +new Date
            // ) {
            //     res.sendStatus(401);
            //     return;
            // }
            let filePath = await text2media(req.query.text === undefined ? 'YEP' : req.query.text);
            if (filePath.endsWith('mp4')) {
                fs.stat(filePath, (err, stat) => {

                    // Handle file not found
                    if (err !== null && err.code === 'ENOENT') {
                        res.sendStatus(404);
                    }

                    const fileSize = stat.size
                    const range = req.headers.range

                    if (range) {
                        console.log("RECEIVED RANGE");

                        const parts = range.replace(/bytes=/, "").split("-");

                        const start = parseInt(parts[0], 10);
                        const end = parts[1] ? parseInt(parts[1], 10) : fileSize - 1;

                        const chunkSize = (end - start) + 1;
                        const file = fs.createReadStream(filePath, {start, end});
                        const head = {
                            'Content-Range': `bytes ${start}-${end}/${fileSize}`,
                            'Accept-Ranges': 'bytes',
                            'Content-Length': chunkSize,
                            'Content-Type': 'video/mp4',
                        }

                        res.writeHead(206, head);
                        file.pipe(res);
                    } else {
                        console.log("DID NO RANGE");
                        const head = {
                            'Content-Length': fileSize,
                            'Content-Type': 'video/mp4',
                        }

                        res.writeHead(200, head);
                        fs.createReadStream(filePath).pipe(res);
                    }
                });
            } else {
                res.sendFile(filePath);
            }
        });
    }
}