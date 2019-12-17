import ApiModule from "../ApiModule.mjs";
import cacher from './Cacher.mjs';
import youtube from './Youtube.mjs'
import path from "path";
import fs from 'fs';
import Log from "../Log";

export default class VmModule extends ApiModule {
    setRoutes(app, _, params) {
        cacher.songDirectory = params.directory;

        app.get('/stream', async (req, res) => {
            let query = req.query.query;

            let file = cacher.toPath(query);
            let exists = await cacher.fileExists(file);
            if (exists) {
                Log.l('VueMusic', "Streaming file", query);
                res.sendFile(path.resolve(file));
                return;
            }
            cacher.cacheIfNotExists(query).then(() => {
                Log.l("VueMusic","cacheIfNotExists finished")
            });

            Log.l('VueMusic', "Streaming YouTube", query);
            let results = await youtube.search(query, 1);

            await youtube.stream(req, res, results[0].id);
        });

        app.get('/download', async (req, res) => {
            let query = req.query.query;

            await cacher.cacheIfNotExists(query);

            let filePath = path.resolve(cacher.toPath(query));
            await res.download(filePath, path.basename(filePath));
        });
    }
}