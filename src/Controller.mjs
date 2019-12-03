import express from 'express';
import https from "https";
import bodyParser from "body-parser";
import cors from "cors";
import fs from 'fs';
import path from 'path';
import VmModule from "./vue-music/VmModule";
import BerberModule from "./berber-api/BerberModule";
import Log from "./Log";


class Controller {
    constructor() {
        this.app = express();
        this.app.use(cors());
        this.app.use(bodyParser.json());

        this.modules = [
            new VmModule(),
            new BerberModule(),
        ];
        this.setRoutes();
    }

    setRoutes() {
        for (let module of this.modules)
            module.setRoutes(this.app);
    }

    static getHttpsCredentials(key, cert) {
        try {
            return {
                key: fs.readFileSync(key),
                cert: fs.readFileSync(cert),
            }
        } catch (e) {
            return false;
        }
    }

    start(port = 3000, key, cert) {
        let credentials = Controller.getHttpsCredentials(key, cert);
        if (credentials) {
            const httpsServer = https.createServer(credentials, this.app);
            httpsServer.listen(port, () => Log.l('Controller', `HTTPS app listening on port ${port}!`));
        } else {
            Log.w('Controller',"Could not get HTTPS credentials, switching to HTTP");
            this.app.listen(port, () => Log.l('Controller', `HTTP app listening on port ${port}!`));
        }
    }
}

export default new Controller();