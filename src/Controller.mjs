import express from 'express';
import https from "https";
import http from "http";
import socketIo from "socket.io";
import bodyParser from "body-parser";
import cors from "cors";
import fs from 'fs';
import BerberModule from "./berber-api/BerberModule.mjs";
import Log from "./Log.mjs";
import StatusModule from "./status/StatusModule.mjs";
import Utils from "./Utils.mjs";
import MediaDownloadModule from "./media-download/MediaDownloadModule.mjs";
import ReverseProxyModule from "./reverse-proxy/ReverseProxyModule.mjs";
import SignalModule from 'multi-signal-server'
import VM5Module from "./vue-music-5/VM5Module.mjs";


class Controller {
    constructor() {
        this.app = express();
        this.app.use(cors());
        this.app.use(bodyParser.json());

        this.modules = [
            new BerberModule(),
            new SignalModule(['peercord']),
            new StatusModule(),
            new MediaDownloadModule(),
            new ReverseProxyModule(),
        ];
    }

    setRoutes() {
        this.app.post('/auth/', async (req, res) => {
            let auth = await Utils.checkAuthorization(req);
            res.send(auth);
        });

        for (let module of this.modules) {
            Log.l("Controller", 'Initialized ' + module.constructor.name);
            module.setRoutes(this.app, this.io, this.params);
        }
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

    start(port = 3000, key, cert, params) {
        this.params = params;
        let credentials = Controller.getHttpsCredentials(key, cert);
        let server;
        if (credentials) {
            server = https.createServer(credentials, this.app);
        } else {
            server = http.createServer(this.app);
            Log.w('Controller', "Could not get HTTPS credentials, switching to HTTP");
        }
        this.io = socketIo(server);
        this.setRoutes();
        server.listen(port, () => Log.l('Controller', `${credentials ? 'HTTPS' : 'HTTP'} server listening on port ${port}!`));
    }
}

export default new Controller();