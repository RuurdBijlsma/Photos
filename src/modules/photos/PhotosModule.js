import ApiModule from "../../ApiModule.js";
import {watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import config from "../../../res/photos/config.json";
import path from "path";

const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
    }

    async setRoutes(app, io, db) {
        await watchAndSynchronize()
        console.log("Watching and synchronizing");

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
            if (filePath === null)
                return res.sendStatus(404);
            let file = path.resolve(path.join(basePath, filePath));
            res.sendFile(file, {acceptRanges: true});
        });
    }
}
