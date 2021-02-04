import ApiModule from "../../ApiModule.js";
import path from 'path';
import crypto from 'crypto';
import plexCredentials from '../../../res/download/credentials.json';
import PlexAPI from "plex-api";
import Auth from "../../database/Auth.js";
import ffmpeg from 'fluent-ffmpeg'
import fs from 'fs'
import {SubtitleParser} from "matroska-subtitles";

const parser = new SubtitleParser()

const client = new PlexAPI({
    hostname: 'ruurdbijlsma.com',
    port: 32400,
    ...plexCredentials,
});

export default class MediaDownloadModule extends ApiModule {
    constructor() {
        super();
        this.tokens = {};
        this.mediaPath = process.platform !== 'win32' ?
            '/mnt/hdd/media/complete' : path.resolve('res/media')
    }

    setRoutes(app, io) {
        app.post('/library/deck', async (req, res) => {
            if (!await Auth.checkRequest(req))
                return res.sendStatus(401);

            let deck = await client.query('/library/sections/1/onDeck')
            let seasons = deck.MediaContainer.Metadata;

            res.send(seasons);
        });
        app.post('/library/shows/', async (req, res) => {
            if (!await Auth.checkRequest(req))
                return res.sendStatus(401);

            if (req.query.hasOwnProperty('show')) {

                let showQuery = req.query.show;
                let seasons = (await client.query(showQuery)).MediaContainer.Metadata;
                await Promise.all(seasons.map(async season => {
                    let episodes = (await client.query(season.key)).MediaContainer.Metadata;
                    if (episodes !== undefined)
                        season.info = (await client.query(episodes[0].parentKey)).MediaContainer.Metadata[0];
                    else
                        season.info = {thumb: undefined, art: undefined}
                }));

                res.send(seasons);

            } else if (req.query.hasOwnProperty('season')) {

                let seasonQuery = req.query.season;
                let episodes = (await client.query(seasonQuery)).MediaContainer.Metadata;
                res.send(episodes);

            } else {

                let result = await client.query('/library/sections/1/folder/');
                let shows = result.MediaContainer.Metadata;
                await Promise.all(shows.map(async show => {
                    let seasons = (await client.query(show.key)).MediaContainer.Metadata;
                    if (!seasons) {
                        show.info = {thumb: undefined, art: undefined};
                        return
                    }
                    let episodes = (await client.query(seasons[0].key)).MediaContainer.Metadata;
                    if (!episodes) {
                        show.info = {thumb: undefined, art: undefined};
                        return
                    }

                    show.info = (await client.query(episodes[0].grandparentKey)).MediaContainer.Metadata[0];
                }));
                res.send(shows);
            }
        });

        app.post('/library/movies/', async (req, res) => {
            if (!await Auth.checkRequest(req))
                return res.sendStatus(401);

            let result = await client.query('/library/sections/3/folder/');
            let movies = result.MediaContainer.Metadata;
            await Promise.all(movies.map(async movie => {
                let movieResult = await client.query(movie.key)
                if (!movieResult.MediaContainer.Metadata) {
                    movie.info = {thumb: undefined, art: undefined};
                    return
                }
                movie.info = movieResult.MediaContainer.Metadata[0];
            }));
            res.send(movies);
        });

        app.get('/library/image', async (req, res) => {
            let image = req.query.image;
            if (isNaN(+image.substr(0, 1))) {
                res.send(false);
                return;
            }

            let blackListParts = ['#', '$', '&', '?'];
            for (let part of blackListParts)
                if (image.includes(part)) {
                    console.warn("In blacklist");
                    res.send(false);
                    return;
                }
            let whiteListParts = ['/thumb/'];
            for (let part of whiteListParts)
                if (!image.includes(part)) {
                    console.warn("Not in whitelist");
                    res.send(false);
                    return;
                }

            let result = await client.query('/library/metadata/' + image);
            if (!(result instanceof Buffer)) {
                res.send(false);
                return;
            }
            res.send(result);
        });

        app.post('/filetoken/', async (req, res) => {
            if (!await Auth.checkRequest(req))
                return res.sendStatus(401);

            let file = req.query.file.replace(/\/media\/data\//, this.mediaPath);
            let token = await this.getToken();
            this.tokens[token] = file;
            setTimeout(() => {
                if (this.tokens.hasOwnProperty(token))
                    delete this.tokens[token];
            }, 10000);
            res.send({token});
        });

        app.get('/file/', async (req, res) => {
            if (!req.query.hasOwnProperty('token')) {
                res.send("No token provided");
                return;
            }
            let token = req.query.token;
            if (this.tokens.hasOwnProperty(token)) {
                let filePath = path.resolve(this.tokens[token]);
                delete this.tokens[token];
                await res.download(filePath, path.basename(filePath));
            } else {
                res.send('Token incorrect');
            }
        });

        app.get('/subs/', async (req, res) => {
            if (process.platform !== 'win32')
                return res.sendStatus(401);

            let filePath = '/media/data/one/1.mkv'
            filePath = path.join(this.mediaPath, filePath.replace(/\/media\/data\//, ''));


            // first an array of subtitle track information is emitted
            parser.once('tracks', (tracks) => console.log(tracks))

            // afterwards each subtitle is emitted
            parser.on('subtitle', (subtitle, trackNumber) =>
                console.log('Track ' + trackNumber + ':', subtitle))

            fs.createReadStream(filePath).pipe(parser)
        })

        app.get('/filetest/', async (req, res) => {
            if (process.platform !== 'win32')
                return res.sendStatus(401);

            let filePath = '/media/data/one/1.mkv'
            filePath = path.join(this.mediaPath, filePath.replace(/\/media\/data\//, ''));

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
                        'Content-Type': 'video/x-matroska',
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
        });
    }

    getToken() {
        return new Promise((resolve, reject) => {
            crypto.randomBytes(48, (err, buffer) => {
                if (err) {
                    reject(err);
                    return;
                }

                resolve(buffer.toString('hex'));
            });
        });
    }
}