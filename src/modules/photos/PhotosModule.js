import ApiModule from "../../ApiModule.js";
import {watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import config from "../../../res/photos/config.template.json";
import path from "path";
import mime from 'mime-types'
import {searchMediaRanked} from "../../database/models/photos/mediaUtils.js";

const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
    }

    async setRoutes(app, io, db) {
        await watchAndSynchronize()
        console.log("Watching and synchronizing");

        app.get('/photo/search/', async (req, res) => {
            let query = req.query.q;
            let result = await searchMediaRanked({
                query,
                includedFields: ['id', 'filename'],
            })
            res.send(result);
        })

        app.get('/photo/:id/:size', async (req, res) => {
            const id = req.params.id;
            let item = await MediaItem.findOne({where: {id}});
            if (item === null)
                return res.sendStatus(404);
            let basePath = req.params.size === 'full' ? config.media : config.thumbnails;
            let filePath = {
                full: item.filePath,
                small: item.smallThumbPath,
                big: item.bigThumbPath,
                webm: item.webmPath,
            }[req.params.size];
            if (filePath === null || filePath === undefined)
                return res.sendStatus(404);
            let file = path.resolve(path.join(basePath, filePath));
            if (req.params.size === 'webm')
                res.contentType('video/webm');
            if (req.params.size === 'full' && item.type === 'video') {
                let mimeType = mime.lookup(path.extname(item.filename));
                res.contentType(mimeType);
            }
            res.sendFile(file, {acceptRanges: true});
        });
    }
}
