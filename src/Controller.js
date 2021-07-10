import express from 'express';
import http from "http";
import bodyParser from "body-parser";
import cors from "cors";
import Log from "./Log.js";
import AuthModule from './modules/auth/AuthModule.js';
import PhotosModule from "./modules/photos/PhotosModule.js";
import cred from "../res/db-config.json"
import Database from "./database/Database.js";
import fileUpload from "express-fileupload";
const {dbUser, dbName} = cred;
const console = new Log("Controller");

class Controller {
    constructor() {
        this.app = express();
        this.app.use(cors());
        this.app.use(fileUpload());
        this.app.use(bodyParser.json());

        this.modules = [
            new AuthModule(),
            new PhotosModule(),
        ];
    }

    setRoutes() {
        for (let module of this.modules) {
            module.setRoutes(this.app, this.db);
            console.log('Initialized ' + module.constructor.name);
        }
    }

    async start(port = 3000) {
        let server = http.createServer(this.app);
        console.log("Initializing DB connection with ", {dbName, dbUser});
        this.db = await Database.initDb();

        this.setRoutes();

        server.listen(port, () =>
            console.log(`HTTP server listening on port ${port}!`)
        );
    }
}

export default new Controller();
