import ApiModule from "../ApiModule";
import cacher from './Cacher';
import youtube from './Youtube'
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
            res.sendFile(path.resolve(cacher.toPath(query)));
        });
    }
}