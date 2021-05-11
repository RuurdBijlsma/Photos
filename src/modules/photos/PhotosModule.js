import ApiModule from "../../ApiModule.js";
import {watchAndSynchronize} from "./watchAndSynchonize.js";
import Log from "../../Log.js";

const console = new Log("PhotosModule");

export default class PhotosModule extends ApiModule {
    constructor() {
        super();
        watchAndSynchronize().then(() => console.log("Watching and synchronizing"))
    }

    setRoutes(app, io, db) {
        app.get('/photo/:fileName', async (req, res) => {
            console.log('get photo', req.query.fileName);
            res.send("asdf");
        });
    }
}
