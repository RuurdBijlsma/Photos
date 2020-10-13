import ApiModule from "../../ApiModule.js";
import ytdl from 'ytdl-core';

export default class VM5Module extends ApiModule {
    constructor() {
        super();
    }

    setRoutes(app) {
        app.post('/ytdl', async (req, res) => {
            res.send(await ytdl.getInfo(req.query.id, {
                quality: 'highestaudio',
                filter: 'audioonly',
            }))
        });
    }
}