import ApiModule from "../../ApiModule.js";
import {processMedia, watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import config from "../../../res/photos/config.json";
import path from "path";
import mime from 'mime-types'
import {
    changeItemDate,
    dropMediaItem, getBoundingBox, getGlossary,
    getMediaById,
    getMonthPhotos,
    getPhotoMonths, getPhotosForMonth, getPhotosPerDayMonth,
    getRandomLabels,
    getRandomLocations,
    searchMediaRanked
} from "../../database/models/photos/mediaUtils.js";
import express from "express";
import geocode from "./reverse-geocode.js";
import Auth from "../../database/Auth.js";
import sequelize from "sequelize";
import {MediaSuggestion} from "../../database/models/photos/MediaSuggestionModel.js";
import Database from "../../database/Database.js";
import {MediaLocation} from "../../database/models/photos/MediaLocationModel.js";

const {Op} = sequelize;
const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
        this.randomLabels = null;
        this.randomLocations = null;
    }

    async setRoutes(app, io, db) {
        app.use('/photos', express.static(config.thumbnails));

        app.post('/photos/photosInBounds', async (req, res) => {
            try {
                let {minLat, maxLat, minLng, maxLng} = req.body;
                if (!await Auth.checkRequest(req)) return res.sendStatus(401);
                res.send(await MediaItem.findAll({
                    include: {
                        model: MediaLocation,
                        where: {
                            latitude: {
                                [Op.gte]: minLat,
                                [Op.lte]: maxLat,
                            },
                            longitude: {
                                [Op.gte]: minLng,
                                [Op.lte]: maxLng,
                            },
                        },
                        attributes: ['latitude', 'longitude'],
                    },
                    limit: 500,
                    attributes: ['id', 'type','width','height'],
                }));
            } catch (e) {
                res.sendStatus(400);
            }
        });

        app.post('/photos/boundingBox/:place', async (req, res) => {
            let place = req.params.place;
            if (typeof place !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let results = await getBoundingBox(place);
            if (results.length === 0) return res.sendStatus(404);
            let result = results[0];
            if (result.maxlat === null || result.minlat === null || result.maxlng === null || result.minlng === null)
                return res.sendStatus(404);
            res.send(result);
        });

        app.post('/photos/isPlace/:place', async (req, res) => {
            let place = req.params.place;
            if (typeof place !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let result = await MediaSuggestion.findOne({
                where: {type: 'place', text: {[Op.iLike]: `${place}`,},},
                attributes: ['text'],
            })

            res.send({isPlace: result !== null, name: result?.text ?? null});
        });

        app.post('/photos/defineLabel/:label', async (req, res) => {
            let label = req.params.label;
            if (typeof label !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let result = await MediaSuggestion.findOne({
                where: {type: 'label', text: label},
                attributes: ['text'],
            });
            if (result === null)
                return res.send({isLabel: false, glossary: null});
            res.send(getGlossary(label));
        });

        app.post('/photos/changeDate/:id', async (req, res) => {
            if (!isFinite(req.body.date)) return res.sendStatus(400);
            const id = req.params.id;
            if (typeof id !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let item = await MediaItem.findOne({where: {id}});
            if (item === null) return res.sendStatus(404);

            let date = new Date(req.body.date);
            if (isNaN(date.getDate()))
                return res.sendStatus(400);
            try {
                await changeItemDate(item, date);
                res.send(true);
            } catch (e) {
                res.send(false);
            }
        });

        app.post('/photos/reprocess/:id', async (req, res) => {
            const id = req.params.id;
            if (typeof id !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let item = await MediaItem.findOne({where: {id}});
            if (item === null) return res.sendStatus(404);

            let filePath = path.resolve(path.join(config.media, item.filePath));

            let newId = null;
            await Database.db.transaction({}, async transaction => {
                await dropMediaItem(item.id, transaction);
                newId = await processMedia(filePath, 2, transaction);
            });
            res.send({id: newId});
        });

        app.post('/photos/month-photos', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            try {
                let months = req.body.months;
                let result = await Promise.all(
                    months.map(date => getMonthPhotos(...date))
                );
                res.send(result);
            } catch (e) {
                res.sendStatus(500);
            }
        });

        app.post('/photos/months', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            res.send(await getPhotoMonths());
        });

        app.post('/photos/list', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
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
                attributes: ['id', 'type', 'subType', 'durationMs', 'createDate', 'width', 'height']
            });
            res.send(photos);
        })

        app.post('/photos/locations/', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let now = +new Date();
            const refreshEvery = 1000 * 60 * 15;// 15 minutes
            if (!this.randomLocations || this.randomLocations.date + refreshEvery < now) {
                this.randomLocations = {date: now, locations: getRandomLocations(15)};
            }
            let locations = await this.randomLocations.locations;
            res.send(locations);
        });

        app.post('/photos/labels/', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let now = +new Date();
            const refreshEvery = 1000 * 60 * 15;// 15 minutes
            if (!this.randomLabels || this.randomLabels.date + refreshEvery < now) {
                this.randomLabels = {date: now, labels: getRandomLabels(15)};
            }
            let labels = await this.randomLabels.labels;
            res.send(labels);
        });

        app.post('/photos/suggestions', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let query = req.query.q;
            query = query.split(' ').filter(n => n.length > 0).join(' ');

            res.send(await MediaSuggestion.findAll({
                where: {text: {[Op.iLike]: `%${query}%`,},},
                order: [['count', 'DESC']],
                limit: 10,
                attributes: ['text', 'count', 'type'],
            }));
        });

        app.post('/photos/search/', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let query = req.query.q;
            query = query.split(' ').filter(n => n.length > 0).join(' ');

            let queryType = await MediaSuggestion.findOne({
                where: {text: {[Op.iLike]: `${query}`,},},
                attributes: ['text', 'type'],
            });
            let type = null, info = null;
            if (queryType !== null) {
                if (queryType.type === 'place') {
                    type = 'place';
                    info = queryType.text;
                    console.log(info);
                } else if (queryType.type === 'label') {
                    let {isLabel, glossary} = await getGlossary(query);
                    if (isLabel) {
                        info = glossary;
                        type = 'label';
                    }
                } else {
                    type = queryType.type;
                }
            }

            let results = await searchMediaRanked({
                query,
                includedFields: ['id', 'type', 'subType', 'durationMs', 'createDate', 'width', 'height'],
            })
            res.send({results, type, info});
        });

        app.post('/photos/dateSearch/', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let month = +req.query.m;
            let day = +req.query.d;
            if (isFinite(month) && isFinite(day)) {
                res.send(await getPhotosPerDayMonth(day, month));
            } else if (isFinite(month)) {
                res.send(await getPhotosForMonth(month));
            }
        });

        app.post('/photos/:id', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            const id = req.params.id;
            if (!id)
                return res.sendStatus(401);
            let item = await getMediaById(id);
            if (item === null)
                return res.sendStatus(404);
            res.send(item);
        });

        app.get('/photos/full/:id', async (req, res) => {
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
