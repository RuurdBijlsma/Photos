import ApiModule from "../ApiModule.mjs";
import ytdl from 'ytdl-core';

export default class VM5Module extends ApiModule {
    constructor() {
        super();
    }

    setRoutes(app, _, params) {
        app.post('/ytdl', async (req, res) => {
            res.send(await ytdl.getInfo(req.query.id, {
                quality: 'highestaudio',
                filter: 'audioonly',
            }))
        });
    }
}