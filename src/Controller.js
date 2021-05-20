import express from 'express';
import https from "https";
import http from "http";
import socketIo from "socket.io";
import bodyParser from "body-parser";
import cors from "cors";
import fs from 'fs';
import Log from "./Log.js";
import BerberModule from "./modules/berber-api/BerberModule.js";
import StatusModule from "./modules/status/StatusModule.js";
import MediaDownloadModule from "./modules/media-download/MediaDownloadModule.js";
import ReverseProxyModule from "./modules/reverse-proxy/ReverseProxyModule.js";
import SignalModule from 'multi-signal-server'
import VM5Module from "./modules/vue-music-5/VM5Module.js";

import AuthModule from './modules/auth/AuthModule.js';
import TwimoteModule from "./modules/twimote-bot/TwimoteModule.js";
import PhotosModule from "./modules/photos/PhotosModule.js";

import seq from "sequelize";
import cred from "../res/auth/credentials.json"
import Database from "./database/Database.js";

const {Sequelize} = seq;
const {dbUser, dbPass, dbName} = cred;
const console = new Log("Controller");

class Controller {
    constructor() {
        this.app = express();
        this.app.use(cors());
        this.app.use(bodyParser.json());

        this.modules = [
            new BerberModule(),
            new VM5Module(),
            new SignalModule(['peercord']),
            new StatusModule(),
            new MediaDownloadModule(),
            new ReverseProxyModule(),
            new AuthModule(),
            new PhotosModule(),
        ];
        // if (process.platform !== 'win32')
        //     this.modules.push(new TwimoteModule())
    }

    setRoutes() {
        for (let module of this.modules) {
            module.setRoutes(this.app, this.io, this.db);
            console.log('Initialized ' + module.constructor.name);
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

    async start(port = 3000, key, cert, params) {
        this.params = params;
        let credentials = Controller.getHttpsCredentials(key, cert);
        let server;
        if (credentials) {
            server = https.createServer(credentials, this.app);
        } else {
            server = http.createServer(this.app);
            console.warn("Could not get HTTPS credentials, switching to HTTP");
        }
        this.io = socketIo(server);
        console.log("Initializing DB connection with ", {dbName, dbUser});
        this.db = new Sequelize(dbName, dbUser, dbPass, {
            host: 'localhost',
            dialect: 'postgres',
            logging: false,
        });
        await Database.setDb(this.db);

        this.setRoutes();
        server.listen(port, () =>
            console.log(`${credentials ? 'HTTPS' : 'HTTP'} server listening on port ${port}!`)
        );
    }
}

export default new Controller();
