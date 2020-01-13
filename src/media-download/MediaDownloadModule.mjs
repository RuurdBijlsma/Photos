import ApiModule from "../ApiModule.mjs";
import path from 'path';
import Utils from "../Utils.mjs";
import crypto from 'crypto';
import plexCredentials from '../../res/download/credentials.json';
import PlexAPI from "plex-api";

const plexToken = plexCredentials.plexToken;
const client = new PlexAPI({hostname: '192.168.0.133', token: plexToken});


export default class MediaDownloadModule extends ApiModule {
    constructor() {
        super();
        this.baseDir = '/mnt/hdd/media/complete/';
        // this.baseDir = 'res/'; //For testing
        this.tokens = {};
    }

    setRoutes(app, io) {
        app.post('/library/shows/', async (req, res) => {
            if (!await Utils.checkAuthorization(req)) {
                res.send(false);
                return;
            }

            if (req.query.hasOwnProperty('show')) {

                let showQuery = req.query.show;
                let seasons = (await client.query(showQuery)).MediaContainer.Metadata;
                await Promise.all(seasons.map(async season => {
                    let episodes = (await client.query(season.key)).MediaContainer.Metadata;
                    let metaKey = episodes[0].parentKey;
                    season.info = (await client.query(metaKey)).MediaContainer.Metadata;
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
                    let episodes = (await client.query(seasons[0].key)).MediaContainer.Metadata;
                    let metaKey = episodes[0].grandparentKey;
                    show.info = (await client.query(metaKey)).MediaContainer.Metadata;
                }));
                res.send(shows);

            }
        });
        app.post('/library/movies/', async (req, res) => {
            if (!await Utils.checkAuthorization(req)) {
                res.send(false);
                return;
            }

            let result = await client.query('/library/sections/2/folder/');
            let movies = result.MediaContainer.Metadata;
            await Promise.all(movies.map(async movie => {
                movie.info = (await client.query(movie.key)).MediaContainer.Metadata;
            }));
            res.send(movies);
        });

        app.post('/filetoken/', async (req, res) => {
            if (!await Utils.checkAuthorization(req)) {
                res.send(false);
                return;
            }
            let file = req.query.file.replace(/\/data\//, '');
            let token = await this.getToken();
            this.tokens[token] = file;
            setTimeout(() => {
                if (this.tokens.hasOwnProperty(token))
                    delete this.tokens[token];
            }, 10000);
            res.send(token);
        });

        app.get(/file/, async (req, res) => {
            if (!req.query.hasOwnProperty('token')){
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