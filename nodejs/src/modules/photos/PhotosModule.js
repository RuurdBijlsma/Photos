import ApiModule from "../../ApiModule.js";
import {processMedia, watchAndSynchronize} from "./watchAndSynchonize.js";
import Clog from "../../Clog.js";
import {Media} from "../../database/models/MediaModel.js";
import config from "../../config.js";
import path from "path";
import mime from 'mime-types'
import {
    autoFixDate,
    changeMediaDate, createZip, deleteFile, dropMedia, getAlbums,
    getBoundingBox, getGlossary,
    getMediaById,
    getMonthPhotos,
    getPhotoMonths, getPhotosForMonth, getPhotosPerDayMonth,
    getRandomLabels,
    getRandomLocations, reprocess,
    searchMediaRanked, uploadFile
} from "../../database/models/mediaUtils.js";
import express from "express";
import geocode from "./reverse-geocode.js";
import Auth from "../../database/Auth.js";
import sequelize from "sequelize";
import {Suggestion} from "../../database/models/SuggestionModel.js";
import Database from "../../database/Database.js";
import {Location} from "../../database/models/LocationModel.js";
import {Blocked} from "../../database/models/BlockedModel.js";
import {batchSize, checkFileExists, getToken, isDate} from "../../utils.js";
import {Log} from "../../database/models/LogModel.js";
import DbInfo from "../../database/DbInfo.js";
import {rotateImage} from "./transcode.js";
import fs from "fs";
import {Album} from "../../database/models/AlbumModel.js";
import {google} from "googleapis";
import Photos from "googlephotos";

const {Op} = sequelize;
const console = new Clog("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
        this.randomLabels = null;
        this.randomLocations = null;
        this.googleAuths = {};
    }

    async setRoutes(app, db) {
        if (config.hostThumbnails)
            app.use('/photo', express.static(config.thumbnails));

        app.post('/photos/importAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let filenames = req.body.filenames;
            let name = req.body.name;
            if (typeof name !== 'string' || !Array.isArray(filenames))
                return res.sendStatus(400);
            let medias = await Promise.all(
                filenames.map(filename => Media.findOne({where: {filename}}))
            );
            let foundMedias = medias.filter(m => !!m);
            let successes = medias.map(m => +!!m);
            let success = successes.reduce((a, b) => a + b) > 0;

            let album;
            if (success) {
                album = await Album.create({
                    id: await getToken(20),
                    name,
                });
                await album.addMedia(foundMedias);
            }

            res.send({albumId: album?.id ?? null, successes, success});
        });

        app.post('/photos/googleAuthTokens', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            const oauth2Client = this.googleAuths[req.body.clientId];

            try {
                const {tokens} = await oauth2Client.getToken(req.body.code);
                oauth2Client.setCredentials(tokens);
                res.send(tokens);
            } catch (e) {
                console.warn('error getting auth tokens', e);
                res.sendStatus(500);
            }
        });

        app.post('/photos/googleAuthUrl', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            const oauth2Client = new google.auth.OAuth2(req.body.clientId, req.body.clientSecret, req.body.redirectUrl);
            this.googleAuths[req.body.clientId] = oauth2Client;

            const scopes = [Photos.Scopes.READ_ONLY, Photos.Scopes.SHARING];
            const url = oauth2Client.generateAuthUrl({
                // 'online' (default) or 'offline' (gets refresh_token)
                access_type: 'offline',
                scope: scopes,
            });

            res.send({url});
        });

        app.post('/photos/renameAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let album = await Album.findOne({where: {id: req.body.id}});
            await album.update({name: req.body.name});
            res.send(true);
        });

        app.post('/photos/deleteAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let album = await Album.findOne({where: {id: req.body.id}});
            await album.destroy();
            res.send(true);
        });

        app.post('/photos/removeFromAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);
            try {
                let album = await Album.findOne({where: {id: req.body.id}});
                if (album === null) return res.sendStatus(404);
                await album.removeMedia(await Media.findAll({
                    where: {id: {[Op.in]: ids}}
                }));
                res.send(true);
            } catch (e) {
                console.warn('remove from album', e);
                res.sendStatus(500);
            }
        });

        app.post('/photos/addToAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            try {
                let album = await Album.findOne({where: {id: req.body.id}});
                let ids = req.body.ids;
                if (!Array.isArray(ids)) return res.sendStatus(400);
                await album.addMedia(await Media.findAll({
                    where: {id: {[Op.in]: ids}}
                }));
                res.send(true);
            } catch (e) {
                console.warn('addToAlbum', e);
                res.sendStatus(500);
            }
        });

        app.post('/photos/createAlbum', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            if (typeof req.body.name !== 'string')
                return res.sendStatus(400);

            try {
                let album = await Album.create({
                    id: await getToken(20),
                    name: req.body.name,
                });
                let ids = req.body.ids;
                if (Array.isArray(ids))
                    await album.addMedia(await Media.findAll({
                        where: {id: {[Op.in]: ids}}
                    }))
                res.send({id: album.id});
            } catch (e) {
                console.warn('createAlbum', e);
                res.sendStatus(500);
            }
        });

        app.post('/photos/getAlbums', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            res.send(await getAlbums());
        });

        app.get('/photos/album/:id', async (req, res) => {
            // if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            if (typeof req.params.id !== 'string') return res.sendStatus(400);
            let sort = req.query.sort;
            if (!sort)
                sort = 'createDate asc';
            sort = sort.split(' ');
            if (sort.length !== 2) return res.sendStatus(400);
            let [column, order] = sort;
            order = order.toUpperCase();
            if (!['ASC', 'DESC'].includes(order)) return res.sendStatus(400);
            let sqlOrder;
            if (column === 'added') {
                sqlOrder = [[sequelize.literal(`"Media->AlbumMedia"."createdAt"`), order]]
            } else if (column === 'createDate') {
                sqlOrder = [[Media, 'createdAt', order]];
            }
            try {
                let result = await Album.findOne({
                    where: {id: req.params.id},
                    include: [{
                        model: Media,
                        attributes: ['id', 'type', 'subType', 'durationMs', 'createDateString', 'width', 'height'],
                    }],
                    order: sqlOrder,
                });
                if (result === null)
                    return res.sendStatus(404);

                res.send(result);
            } catch (e) {
                console.warn('get album', e);
                res.sendStatus(500);
            }
        });

        app.post('/photos/getRestoreOptions', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            res.send((await fs.promises.readdir(config.backups)).reverse());
        });

        app.post('/photos/restoreDb', async (req, res) => {
            let filename = req.body.file;
            if (typeof filename !== 'string')
                return res.status(400).send('filename in body is wrong.');
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            await Database.restore(path.join(config.backups, filename));
            res.send(true);
        });

        app.post('/photos/backupDb', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            await Database.backup('manual');
            res.send(true);
        });

        app.post('/photos/clearErrors', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            await Blocked.destroy({where: {reason: 'error'}});
            res.send(true);
        });

        app.post('/photos/clearTrash', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            await Blocked.destroy({where: {reason: 'deleted'}});
            res.send(true);
        });

        app.post('/photos/rotateImage', async (req, res) => {
            let id = req.body.id;
            let angle = req.body.angle;
            let saveCopy = req.body.copy;
            if (typeof id !== 'string' || typeof angle !== "number" || typeof saveCopy !== "boolean")
                return res.status(400).send('Angle, id or saveCopy in body are wrong.');
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let media = await Media.findOne({where: {id}});
            if (media === null) return res.sendStatus(404);

            let newFileName = media.filePath;
            if (saveCopy) {
                let ext = path.extname(media.filePath);
                let baseFile = media.filePath.substr(0, media.filePath.length - ext.length);
                let i = 1;
                do {
                    newFileName = `${baseFile}(${i++})${ext}`;
                } while (await checkFileExists(path.join(config.media, newFileName)));
            }
            try {
                let success = await rotateImage(path.join(config.media, media.filePath), angle, path.join(config.media, newFileName));
                if (success) {
                    if (!saveCopy)
                        await dropMedia(id);
                    let newId = await processMedia(path.join(config.media, newFileName));
                    if (newId === false)
                        return res.send({success: false, id: null});
                    res.send({success: true, id: newId});
                } else {
                    res.send({success: false, id: null});
                }
            } catch (e) {
                console.warn(`Couldn't rotate image ${media.filePath}`, e.message);
                res.send({success: false, id: null});
            }
        });

        app.post('/photos/logs', async (req, res) => {
            res.send(await Log.findAll({
                where: {
                    LogSessionId: DbInfo.session,
                },
                order: sequelize.col('createdAt'),
            }));
        });

        app.post('/photos/upload', async (req, res) => {
            if (!req.files || Object.keys(req.files).length === 0)
                return res.status(400).send('No files were uploaded.');

            console.log('Receiving file upload');
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

        app.post('/photos/batchDownload', async (req, res) => {
            let ids = req.body.ids;
            if (!Array.isArray(ids)) return res.sendStatus(400);
            if (req.body.albumId) {
                ids = await Auth.checkAlbumAuth(req, ids);
                if (ids.length === 0) return res.sendStatus(401);
            } else {
                if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            }

            let items = await Media.findAll({
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
            let item = await Media.findOne({where: {id}});
            if (item === null) return res.sendStatus(404);

            let date = new Date(req.body.date);
            if (isNaN(date.getDate()))
                return res.sendStatus(400);
            try {
                await changeMediaDate(item, date);
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
            res.send(await Blocked.findAll());
        });

        app.post('/photos/totalBounds', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            let result = await Database.db.query(`
                select min(latitude) as minlat, max(latitude) maxlat, min(longitude) minlng, max(longitude) maxlng
                from "Locations"
            `, {type: sequelize.QueryTypes.SELECT});
            if (Array.isArray(result) && result.length > 0)
                res.send(result[0])
            else
                res.send(null);
        });

        app.post('/photos/searchTip', async (req, res) => {
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let result = await Database.db.query(`
                select text, count::FLOAT / (select max(count) from "Suggestions") + random() * 5 as rcount
                from "Suggestions"
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
                res.send(await Media.findAll({
                    include: {
                        model: Location,
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

            let result = await Suggestion.findOne({
                where: {type: 'place', text: {[Op.iLike]: `${place}`,},},
                attributes: ['text'],
            })

            res.send({isPlace: result !== null, name: result?.text ?? null});
        });

        app.post('/photos/defineLabel/:label', async (req, res) => {
            let label = req.params.label;
            if (typeof label !== 'string') return res.sendStatus(400);
            if (!await Auth.checkRequest(req)) return res.sendStatus(401);

            let result = await Suggestion.findOne({
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
            let item = await Blocked.findOne({where: {filePath}});
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
            let photos = await Media.findAll({
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

            res.send(await Suggestion.findAll({
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
            const subTypes = ['portrait', 'vr', 'slomo', 'animation'];

            let dateMeta = isDate(query);
            let meta = subTypes.includes(query) ? {type: 'subType'} :
                dateMeta.type !== 'none' ? {type: 'date'} :
                    await Suggestion.findOne({
                        where: {text: {[Op.iLike]: `${query}`}},
                        attributes: ['text', 'type'],
                    });
            let type = null, info = null;
            if (meta !== null) {
                if (meta.type === 'place') {
                    type = 'place';
                    info = meta.text;
                    console.log(info);
                } else if (meta.type === 'label') {
                    let {isLabel, glossary} = await getGlossary(query);
                    if (isLabel) {
                        info = glossary;
                        type = 'label';
                    }
                } else {
                    type = meta.type;
                }
            }

            let results;
            if (type === 'subType') {
                results = await Media.findAll({where: {subType: query}});
            } else if (type === 'date') {
                if (dateMeta.type === 'month') {
                    results = await getPhotosForMonth(dateMeta.month);
                } else if (dateMeta.type === 'dayMonth') {
                    results = await getPhotosPerDayMonth(dateMeta.day, dateMeta.month);
                }
            } else {
                results = await searchMediaRanked({
                    query,
                    includedFields: ['id', 'type', 'subType', 'durationMs', 'createDateString', 'width', 'height'],
                });
            }
            res.send({results, type, info});
        });

        app.post('/photos/:id', async (req, res) => {
            const id = req.params.id;
            if (!id)
                return res.sendStatus(401);
            if (req.body.albumId) {
                let ids = await Auth.checkAlbumAuth(req, [id]);
                if (ids.length === 0) return res.sendStatus(401);
            } else {
                if (!await Auth.checkRequest(req)) return res.sendStatus(401);
            }
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
            let item = await Blocked.findOne({where: {id}});
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
            let item = await Media.findOne({where: {id}});
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
