import ApiModule from "../ApiModule.mjs";
import path from "path";
import fs from 'fs';
import Log from "../Log.mjs";
import MusicDownloader from "./MusicDownloader.mjs";

export default class VM5Module extends ApiModule {
    constructor() {
        super();
        this.downloader = new MusicDownloader('./', './', './');
    }

    setRoutes(app, _, params) {
        this.downloader.directories.music = params.directory;

        app.post('/trackurl', async (req, res) => {
            try {
                let {track, apiKey} = req.body;

                let wantedIndex = +req.query.index || 0;
                let urlResult = await (this.downloader.getTrackUrls(track, wantedIndex, apiKey).next());
                res.send(urlResult.value);
            } catch (e) {
                console.log(e);
                res.status(400).send("Invalid request body");
            }
        });
    }
}