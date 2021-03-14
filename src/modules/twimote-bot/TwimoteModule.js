import ApiModule from "../../ApiModule.js";
import {text2media, getFileType} from './twimote.js';
import fs from "fs";
import TelegramBot from "node-telegram-bot-api";
import tokens from "../../../res/twimote/tokens.json";
import Utils from "../../Utils.js";

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
            console.log(id);
            let token = await Utils.getToken();
            this.tokens[token] = {expiryDate: (new Date) + 10000};
            setTimeout(() => {
                if (this.tokens.hasOwnProperty(token))
                    delete this.tokens[token];
            }, 10000);

            let fileType = getFileType(query);
            if (fileType === 'mp4') {
                bot.answerInlineQuery(id, [{
                    type: 'mpeg4_gif',
                    id: Math.floor(Math.random() * 10000000),
                    mpeg4_url: 'https://api.ruurd.dev/twimote?text=' + encodeURIComponent(query),
                    thumb_url: 'https://i.picsum.photos/id/167/200/300.jpg?hmac=ZAuGlRPlSv0i_JnJr4FFW-OPsVz5bTx8mAI_qUYP_bM',
                }]);
            } else {
                let url = 'https://api.ruurd.dev/twimote?text=' + encodeURIComponent(query);
                console.log(`photo url: ${url}`);
                bot.answerInlineQuery(id, [{
                    type: 'photo',
                    id: Math.floor(Math.random() * 10000000),
                    photo_url: url,
                    thumb_url: 'https://i.picsum.photos/id/167/200/300.jpg?hmac=ZAuGlRPlSv0i_JnJr4FFW-OPsVz5bTx8mAI_qUYP_bM',
                }]);
            }
            // bot.answerInlineQuery(id, [{
            //     type: 'article',
            //     id: Math.floor(Math.random() * 10000000),
            //     title: 'hello',
            //     input_message_content: {
            //         message_text: 'https://example.com',
            //     },
            // }])
        });

        bot.onText(/\/echo (.+)/, (msg, match) => {
            // 'msg' is the received Message from Telegram
            // 'match' is the result of executing the regexp above on the text content
            // of the message
            console.log("echo", msg, match);

            const chatId = msg.chat.id;
            const resp = match[1]; // the captured "whatever"

            // send back the matched "whatever" to the chat
            // bot.sendMessage(chatId, `${chatId} ${resp}`);
            let video = fs.createReadStream('./res/bonk.gif');
            bot.sendAnimation(chatId, video);
        });

        bot.on('message', (msg) => {
            console.log("message", msg);
            const chatId = msg.chat.id;

            // send a message to the chat acknowledging receipt of their message
            bot.sendMessage(chatId, 'Received your message');
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
            let filePath = await text2media(req.query.text === undefined ? 'YEP' : req.query.text, res).then();
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