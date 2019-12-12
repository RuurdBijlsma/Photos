import ApiModule from "../ApiModule.mjs";
import cacher from './Cacher.mjs';
import youtube from './Youtube.mjs'
import path from "path";

export default class VmModule extends ApiModule {
    setRoutes(app, _, params) {
        cacher.songDirectory = params.directory;

        app.get('/stream', async (req, res) => {
            let results = await youtube.search(req.query.query, 1);

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