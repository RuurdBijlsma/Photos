import express from 'express';
import https from "https";
import http from "http";
import socketIo from "socket.io";
import bodyParser from "body-parser";
import cors from "cors";
import fs from 'fs';
import VmModule from "./vue-music/VmModule";
import BerberModule from "./berber-api/BerberModule";
import Log from "./Log";
import SignalModule from "./signal-server/SignalModule";

//TODO:
//SOCKET IO implement
//SignalServer implement


class Controller {
    constructor() {
        this.app = express();
        this.app.use(cors());
        this.app.use(bodyParser.json());

        this.modules = [
            new VmModule(),
            new BerberModule(),
            new SignalModule(),
        ];
    }

    setRoutes() {
        for (let module of this.modules) {
            Log.l("Controller", module.constructor.name, " initialized");
            module.setRoutes(this.app, this.io);
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

    start(port = 3000, key, cert) {
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
        server.listen(port, () => console.log(`${credentials?'HTTPS':'HTTP'} server listening on port ${port}!`));
    }
}

export default new Controller();