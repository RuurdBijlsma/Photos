import ApiModule from "../../ApiModule.js";
import {text2media, getTextSize, getFileType, getSuggestions, search} from './twimote.js';
import fs from "fs";
import TelegramBot from "node-telegram-bot-api";
import tokens from "../../../res/twimote/tokens.json";
import {EmoteSticker} from "../../database/models/EmoteStickerModel.js";
import addEmote from "./getEmotes/addEmote.js";
import {Emote} from "../../database/models/EmoteModel.js";
import path from "path";

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
        if (process.platform !== 'win32')
            this.botSetup();
    }

    async botSetup() {
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
                    // let url = `https://api.ruurd.dev/twimote?text=${encodeURIComponent(text)}&type=${type}`;
                    let url = `http://82.73.25.96:3000/twimote?text=${encodeURIComponent(text)}&type=${type}`;
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
                    } else {
                        stickerId = emote.sticker;
                    }
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
            await bot.answerInlineQuery(id, queryAnswers, {cache_time: 10});
        });

        bot.onText(/\/search (.+)/, async (msg, match) => {
            const chatId = msg.chat.id;
            const query = match[1];
            let results = await search(query);
            for (let result of results)
                bot.sendMessage(chatId, `[${result.name}](${result.url})`, {
                    parse_mode: 'MarkdownV2',
                });
        });
        bot.onText(/\/search/, async (msg) => {
            if (msg.text.split(' ').length > 1)
                return;
            await bot.sendMessage(msg.chat.id, `Use \`/search query\` search for emotes containing the string 'query'`, {
                parse_mode: 'MarkdownV2',
            });
        });

        bot.onText(/\/add (.+) (.+)/, async (msg, match) => {
            const chatId = msg.chat.id;
            if (!tokens.canAddEmotes.includes(msg.from.id)) {
                return bot.sendMessage(chatId, "(unauthorized) You can't add emotes >:(");
            }
            const emote = match[1];
            const url = match[2];
            if (emote.includes(' ') || url.includes(' ')) {
                return bot.sendMessage(
                    chatId,
                    "Add command must be followed by emote name then emote url, example: `/add monkaS https://cdn.frankerfacez.com/emoticon/130762/4`",
                    {parse_mode: 'MarkdownV2'},
                );
            }
            bot.sendMessage(chatId, `Trying to add ${emote}...`);
            if (await addEmote(emote, url)) {
                await bot.sendMessage(chatId, `Added ${emote} ${url} ✅`);
            } else {
                await bot.sendMessage(chatId, `Failed to add ${emote} ${url}!`);
            }
        });
        bot.onText(/\/add/, async (msg) => {
            if (msg.text.split(' ').length > 2)
                return;
            await bot.sendMessage(msg.chat.id, `Use \`/add emoteName https://ffz.com/emote.png\` to add existing emotes`, {
                parse_mode: 'MarkdownV2',
            });
        });

        bot.onText(/\/remove (.+)/, async (msg, match) => {
            const chatId = msg.chat.id;
            if (!tokens.canAddEmotes.includes(msg.from.id)) {
                return bot.sendMessage(chatId, "(unauthorized) You can't remove emotes >:(");
            }
            const emoteName = match[1];
            let emote = await Emote.findOne({where: {name: emoteName}});
            if (emote === null) {
                return bot.sendMessage(chatId, `${emoteName} doesn't exist (remove command is case sensitive)`);
            }
            await emote.destroy();
            await bot.sendMessage(chatId, `${emoteName} has been removed`);
        });
        bot.onText(/\/remove/, async (msg) => {
            if (msg.text !== '/remove')
                return;
            await bot.sendMessage(msg.chat.id, `Use \`/remove emoteName\` to remove existing emotes`, {
                parse_mode: 'MarkdownV2',
            });
        });

        bot.onText(/\/edit (.+) (.+)/, async (msg, match) => {
            const chatId = msg.chat.id;
            if (!tokens.canAddEmotes.includes(msg.from.id)) {
                return bot.sendMessage(chatId, "(unauthorized) You can't edit emotes >:(");
            }
            const emoteName = match[1];
            const url = match[2];
            if (emoteName.includes(' ') || url.includes(' ')) {
                return bot.sendMessage(
                    chatId,
                    "Edit command must be followed by emote name then emote url, example: `/add monkaS https://cdn.frankerfacez.com/emoticon/130762/4`",
                    {parse_mode: 'MarkdownV2'},
                );
            }
            let emote = await Emote.findOne({where: {name: emoteName}});
            if (emote === null) {
                return bot.sendMessage(chatId, `${emoteName} doesn't exist (edit command is case sensitive)`);
            }
            await emote.destroy();
            bot.sendMessage(chatId, `Trying to add ${emoteName}...`);
            if (await addEmote(emoteName, url)) {
                await bot.sendMessage(chatId, `Added ${emoteName} ${url} ✅`);
            } else {
                await bot.sendMessage(chatId, `Failed to add ${emoteName} ${url}!`);
            }
        });
        bot.onText(/\/edit/, async (msg) => {
            if (msg.text.split(' ').length > 2)
                return;
            await bot.sendMessage(msg.chat.id, `Use \`/edit emoteName https://ffz.com/emote.png\` to edit existing emotes`, {
                parse_mode: 'MarkdownV2',
            });
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
