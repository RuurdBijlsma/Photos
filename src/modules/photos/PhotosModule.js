import ApiModule from "../../ApiModule.js";
import {watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import config from "../../../res/photos/config.template.json";
import path from "path";
import mime from 'mime-types'
import {searchMediaRanked} from "../../database/models/photos/mediaUtils.js";
import express from "express";

const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
    }

    async setRoutes(app, io, db) {
        app.use('/photo', express.static(config.thumbnails));

        await watchAndSynchronize()
        console.log("Watching, synchronizing, and starting api");

        app.get('/photos/search/', async (req, res) => {
            let query = req.query.q;
            let result = await searchMediaRanked({
                query,
                includedFields: ['id', 'filename', 'type'],
            })
            res.send(result);
        })

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
    }
}
