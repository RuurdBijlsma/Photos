import ApiModule from "../../ApiModule.js";
import {watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import config from "../../../res/photos/config.template.json";
import path from "path";
import mime from 'mime-types'
import {
    getMonthPhotos,
    getPhotoMonths,
    getRandomLabels,
    getRandomLocations,
    searchMediaRanked
} from "../../database/models/photos/mediaUtils.js";
import express from "express";
import geocode from "./reverse-geocode.js";
import Auth from "../../database/Auth.js";

const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
        this.randomLabels = null;
        this.randomLocations = null;
    }

    async setRoutes(app, io, db) {
        app.use('/photo', express.static(config.thumbnails));

        app.post('/photos/month-photos', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            try {
                let months = req.body.months;
                let result = await Promise.all(
                    months.map(date => getMonthPhotos(...date))
                );
                res.send(result);
            } catch (e) {
                res.sendStatus(401);
            }
        });

        app.post('/photos/months', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            res.send(await getPhotoMonths());
        });

        app.post('/photos/list', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            let limit = +req.query.limit;
            if (!isFinite(limit))
                limit = 10;
            let offset = +req.query.offset;
            if (!isFinite(offset))
                offset = 0;
            limit = Math.min(200, limit);
            let photos = await MediaItem.findAll({
                order: [['createDate', 'DESC']],

                limit,
                offset,
                attributes: ['id', 'subType', 'type', 'createDate', 'filename', 'width', 'height']
            });
            res.send(photos);
        })

        app.post('/photos/locations/', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            let now = +new Date();
            const refreshEvery = 1000 * 60 * 15;// 15 minutes
            if (!this.randomLocations || this.randomLocations.date + refreshEvery < now) {
                this.randomLocations = {date: now, locations: getRandomLocations(15)};
            }
            let locations = await this.randomLocations.locations;
            res.send(locations);
        });

        app.post('/photos/labels/', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            let now = +new Date();
            const refreshEvery = 1000 * 60 * 15;// 15 minutes
            if (!this.randomLabels || this.randomLabels.date + refreshEvery < now) {
                this.randomLabels = {date: now, labels: getRandomLabels(15)};
            }
            let labels = await this.randomLabels.labels;
            res.send(labels);
        });

        app.post('/photos/search/', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            let query = req.query.q;
            query = query.split(' ').filter(n => n.length > 0).join(' ');
            let result = await searchMediaRanked({
                query,
                includedFields: ['id', 'subType', 'type', 'createDate', 'filename', 'width', 'height'],
            })
            res.send(result);
        });

        app.get('/photo/full/:id', async (req, res) => {
            const id = req.params.id;
            let item = await MediaItem.findOne({where: {id}});
            if (item === null)
                return res.sendStatus(404);
            let file = path.resolve(path.join(config.media, item.filePath));
            if (item.type === 'video') {
                let mimeType = mime.lookup(path.extname(item.filename));
                res.contentType(mimeType);
            }
            res.sendFile(file, {acceptRanges: true});
        });

        console.log("Initializing geocoder");
        await geocode({latitude: 50, longitude: 5});
        console.log("Initialized geocoder");
        await watchAndSynchronize()
        console.log("Watching and synchronizing Photos");
    }
}
