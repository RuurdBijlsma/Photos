import ApiModule from "../../ApiModule.js";
import {processMedia, watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/MediaItemModel.js";
import config from "../../config.js";
import path from "path";
import mime from 'mime-types'
import {
    autoFixDate,
    changeItemDate, createZip, deleteFile,
    getBoundingBox, getGlossary,
    getMediaById,
    getMonthPhotos,
    getPhotoMonths, getPhotosForMonth, getPhotosPerDayMonth,
    getRandomLabels,
    getRandomLocations, getZipPath, reprocess,
    searchMediaRanked, uploadFile
} from "../../database/models/mediaUtils.js";
import express from "express";
import geocode from "./reverse-geocode.js";
import Auth from "../../database/Auth.js";
import sequelize from "sequelize";
import {MediaSuggestion} from "../../database/models/MediaSuggestionModel.js";
import Database from "../../database/Database.js";
import {MediaLocation} from "../../database/models/MediaLocationModel.js";
import {MediaBlocked} from "../../database/models/MediaBlockedModule.js";
import {batchSize, checkFileExists} from "../../utils.js";

const {Op} = sequelize;
const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
        this.randomLabels = null;
        this.randomLocations = null;
    }

    async setRoutes(app, db) {
        if (config.hostThumbnails)
            app.use('/photo', express.static(config.thumbnails));

        app.post('/photos/upload', async (req, res) => {
            if (!req.files || Object.keys(req.files).length === 0)
                return res.status(400).send('No files were uploaded.');

            let authenticated;
            try {
                let {email, password} = req.body;
                authenticated = await Auth.check(email, password);
            } catch (e) {
                authenticated = false;
            }
            if (!authenticated) return res.sendStatus(401);

            let files = !Array.isArray(req.files.media) ? [req.files.media] : req.files.media;

            let results = [];
            for (let i = 0; i < files.length; i += batchSize) {
                let slice = files.slice(i, i + batchSize);
                console.log(`Uploading files [${i}-${Math.min(files.length, i + batchSize)} / ${files.length}]`);
                try {
                    results.push(...await Promise.all(slice.map(uploadFile)));
                } catch (e) {
                    console.warn('upload error', e);
                }
            }
            res.send({success: true, results});
        });

        app.post('/photos/mapboxToken', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            return user.mapboxToken;
        });

        app.post('/photos/setMapboxToken', async (req, res) => {
            if (typeof req.body.token !== 'string') return res.sendStatus(400);
            let user = await Auth.checkRequest(req);
            if (!user) return res.sendStatus(401);
            await user.update({mapboxToken: req.body.token});
            res.send(true);
        });

        app.get('/photos/zip/:id', async (req, res) => {
            const zipId = req.params.id;
            let zipFile = getZipPath(zipId);
            if (await checkFileExists(zipFile))
                res.sendFile(zipFile);
            else
                res.sendStatus(400);
        });

        app.post('/photos/batchDownload', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);
            console.log('batch download', ids);

            let items = await MediaItem.findAll({
                where: {id: {[Op.in]: ids}}
            });
            let zipId = await createZip(items.map(i => path.join(config.media, i.filePath)));
            res.send({zipId});
        });

        app.post('/photos/batchDelete', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);

            console.log('batch delete', ids);
            let results = [];
            for (let i = 0; i < ids.length; i += batchSize) {
                let slice = ids.slice(i, i + batchSize);
                console.log(`Deleting files [${i}-${Math.min(ids.length, i + batchSize)} / ${ids.length}]`);
                try {
                    results.push(...await Promise.all(slice.map(deleteFile)));
                } catch (e) {
                    console.warn('Deleting error', e);
                }
            }
            let success = results.every(r => r.success);

            res.send({success, results});
        });

        app.post('/photos/batchFixDate', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);

            console.log('fix date', ids);
            let results = [];
            for (let i = 0; i < ids.length; i += batchSize) {
                let slice = ids.slice(i, i + batchSize);
                console.log(`Fixing dates [${i}-${Math.min(ids.length, i + batchSize)} / ${ids.length}]`);
                try {
                    results.push(...await Promise.all(slice.map(autoFixDate)).then(s => s.map(success => ({success}))));
                } catch (e) {
                    console.warn('Fixing dates error', e);
                }
            }
            let success = results.every(r => r.success);

            res.send({success, results});
        });

        app.post('/photos/batchReprocess', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);

            console.log('fix date', ids);
            let results = [];
            for (let i = 0; i < ids.length; i += batchSize) {
                let slice = ids.slice(i, i + batchSize);
                console.log(`Reprocessing images [${i}-${Math.min(ids.length, i + batchSize)} / ${ids.length}]`);
                try {
                    results.push(...await Promise.all(slice.map(reprocess)));
                } catch (e) {
                    console.warn('Reprocessing images error', e);
                }
            }
            let success = results.every(r => r.success);

            res.send({success, results});
        });

        app.post('/photos/deleteItem/:id', async (req, res) => {
            let id = req.params.id;
            if (typeof id !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            try {
                let result = await deleteFile(id);
                if (result.code) return res.sendStatus(result.code);
                res.send(result.success);
            } catch (e) {
                console.warn('deleteFile error', e);
                res.sendStatus(500);
            }
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
            try {
                let result = await reprocess(id);
                if (result.code) return res.sendStatus(result.code);
                res.send({id: result.id});
            } catch (e) {
                res.sendStatus(500);
            }
        });

        app.post('/photos/blockedItems', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            res.send(await MediaBlocked.findAll());
        });

        app.post('/photos/totalBounds', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let result = await Database.db.query(`
                select min(latitude) as minlat, max(latitude) maxlat, min(longitude) minlng, max(longitude) maxlng
                from "MediaLocations"
            `, {type: sequelize.QueryTypes.SELECT});
            if (Array.isArray(result) && result.length > 0)
                res.send(result[0])
            else
                res.send(null);
        });

        app.post('/photos/searchTip', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let result = await Database.db.query(`
                select text, count::FLOAT / (select max(count) from "MediaSuggestions") + random() * 5 as rcount
                from "MediaSuggestions"
                where text != 'instrumentality'
                  and text != 'instrumentation'
                  and text != 'structure'
                  and text != 'device'
                order by rcount desc
                limit 1
           `, {type: sequelize.QueryTypes.SELECT});
            res.send(result?.[0]);
        });

        app.post('/photos/photosInBounds', async (req, res) => {
            try {
                let {minLat, maxLat, minLng, maxLng, startDate, endDate} = req.body;
                startDate = new Date(startDate);
                endDate = new Date(endDate);
                let whereSpread = {};
                if (!isNaN(startDate.getTime()) && !isNaN(endDate.getTime()))
                    whereSpread = {
                        where: {
                            createDate: {
                                [Op.gte]: startDate,
                                [Op.lte]: endDate,
                            }
                        }
                    }
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
                    ...whereSpread,
                    limit: 1000,
                    attributes: ['id', 'type', 'width', 'height'],
                    order: ['id'],
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
            // Add about 1 meter padding
            res.send({
                maxlat: result.maxlat + 0.00001,
                minlat: result.minlat - 0.00001,
                maxlng: result.maxlng + 0.00001,
                minlng: result.minlng - 0.00001,
            });
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

        app.post('/photos/retryProcess', async (req, res) => {
            const filePath = req.body.filePath;
            if (typeof filePath !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let item = await MediaBlocked.findOne({where: {filePath}});
            if (item !== null)
                await item.destroy();
            try {
                let result = await processMedia(path.join(config.media, filePath));
                res.send({success: result !== false, id: result});
            } catch (e) {
                res.sendStatus(500);
            }
        });

        app.post('/photos/months-photos', async (req, res) => {
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

        app.post('/photos/month-photos', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            try {
                let month = req.body.month;
                res.send(await getMonthPhotos(...month));
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
                attributes: ['id', 'type', 'subType', 'durationMs', 'createDateString', 'width', 'height']
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
                includedFields: ['id', 'type', 'subType', 'durationMs', 'createDateString', 'width', 'height'],
            }).then(this.fixMediaArrayDates);
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
            delete item.vector;
            delete item.vectorA;
            delete item.vectorB;
            delete item.vectorC;
            res.send(item);
        });

        app.get('/photos/blocked/:id', async (req, res) => {
            const id = req.params.id;
            let item = await MediaBlocked.findOne({where: {id}});
            if (item === null)
                return res.sendStatus(404);
            let file = path.resolve(path.join(config.media, item.filePath));
            if (item.type === 'video') {
                let mimeType = mime.lookup(path.extname(item.filePath));
                res.contentType(mimeType);
            }
            if (await checkFileExists(file))
                res.sendFile(file, {acceptRanges: true});
            else
                res.sendStatus(404);
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
        console.time("Init geocoder");
        await geocode({latitude: 50, longitude: 5});
        console.log("Initialized geocoder");
        console.timeEnd("Init geocoder");
        await watchAndSynchronize()
        console.log("Watching and synchronizing Photos");
    }
}