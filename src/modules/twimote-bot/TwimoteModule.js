import ApiModule from "../../ApiModule.js";
import {text2media, getTextSize, getFileType, getSuggestions, search} from './twimote.js';
import fs from "fs";
import TelegramBot from "node-telegram-bot-api";
import tokens from "../../../res/twimote/tokens.json";
import {EmoteSticker} from "../../database/models/EmoteStickerModel.js";

// TODO
// Enable tokens for security
// Webhooks
// suggesties
// perma cache text to sticker file id
// add emote with bot command
// fix telegram min height (mainly for single video emotes when height < 100)
// some emotes not centered?

export default class TwimoteModule extends ApiModule {
    constructor() {
        super();
        // if (process.platform === 'win32')
        this.botSetup();
    }

    botSetup() {
        const bot = new TelegramBot(tokens.telegram, {polling: true});
        console.log("Telegram bot is running")

        bot.on('inline_query', async ({id, query}) => {
            if (query === '')
                query = 'YEP';

            let queryText = query.substr(0, 2000);
            let queryAnswers = await Promise.all((await getSuggestions(queryText)).map(async text => {
                let randomID = Math.round(Math.random() * 10000000);
                let type = await getFileType(text);
                let {width, height, duration} = await getTextSize(text);
                if (type === 'mp4') {
                    // let url = `https://api.ruurd.dev/twimote?text=${encodeURIComponent(text)}&r=${randomID}&type=${type}`;
                    let url = `https://api.ruurd.dev/twimote?text=${encodeURIComponent(text)}&type=${type}`;
                    console.log(url);
                    return {
                        type: 'mpeg4_gif',
                        id: randomID,
                        title: text,
                        mpeg4_url: url,
                        mpeg4_width: width,
                        mpeg4_height: height,
                        thumb_url: url,
                        mpeg4_duration: duration,
                        thumb_mime_type: 'video/mp4',
                    };
                } else if (type === 'png') {
                    let emote = await EmoteSticker.findOne({where: {text}});
                    let stickerId;
                    if (emote === null) {
                        let file = await text2media(text);
                        let msg = await bot.sendPhoto(tokens.stickerDump, file);
                        stickerId = msg.photo[msg.photo.length - 1].file_id;
                        EmoteSticker.create({
                            text,
                            sticker: stickerId,
                        }).then(() => console.log('emote added to db'));
                    } else stickerId = emote.sticker;
                    return {
                        type: 'photo',
                        id: randomID,
                        title: text,
                        photo_file_id: stickerId,
                    };
                } else if (type === 'webp') {
                    let emote = await EmoteSticker.findOne({where: {text}});
                    let stickerId;
                    if (emote === null) {
                        let file = await text2media(text);
                        let msg = await bot.sendSticker(tokens.stickerDump, file);
                        stickerId = msg.sticker.file_id;
                        EmoteSticker.create({
                            text,
                            sticker: stickerId,
                        }).then(() => console.log('emote added to db'));
                    } else stickerId = emote.sticker;
                    return {
                        type: 'sticker',
                        id: randomID,
                        title: text,
                        sticker_file_id: stickerId,
                    };
                }
            }));
            await bot.answerInlineQuery(id, queryAnswers);
        });

        bot.onText(/\/search (.+)/, async (msg, match) => {
            // 'msg' is the received Message from Telegram
            // 'match' is the result of executing the regexp above on the text content
            // of the message

            const chatId = msg.chat.id;
            const query = match[1]; // the captured "whatever"
            let suggestions = await search(query);

            // send back the matched "whatever" to the chat
            for (let suggestion of suggestions) {
                bot.sendMessage(chatId, `${suggestion.name} ${suggestion.url}`);
            }
        });

        bot.on('message', async (msg) => {
            // console.log("message", msg);
            // if (!msg.text)
            //     return;
            //
            // await bot.sendMessage(msg.chat.id, "This bot should only be used inline (i.e. @twimotebot YEP)");
        });
    }

    setRoutes(app, io, db) {
        app.get('/twimote/', async (req, res) => {
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