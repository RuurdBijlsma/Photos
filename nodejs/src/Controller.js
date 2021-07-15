import express from 'express';
import http from "http";
import bodyParser from "body-parser";
import cors from "cors";
import Clog from "./Clog.js";
import AuthModule from './modules/auth/AuthModule.js';
import PhotosModule from "./modules/photos/PhotosModule.js";
import Database from "./database/Database.js";
import fileUpload from "express-fileupload";
import {User} from "./database/models/UserModel.js";
import bcrypt from "bcrypt";

const console = new Clog("Controller");

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
        console.log("Initializing DB connection");
        this.db = await Database.initDb();
        let userCount = await User.count();
        if (userCount === 0) {
            let salt = await bcrypt.genSalt(10);
            await User.create({
                name: process.env.UI_NAME ?? 'user',
                email: process.env.UI_EMAIL ?? 'user@gmail.com',
                password: await bcrypt.hash(process.env.UI_PASSWORD ?? '0123456789', salt),
            });
            console.log("Created new user");
        }

        this.setRoutes();

        server.listen(port, () =>
            console.log(`HTTP server listening on port ${port}!`)
        );
    }
}

export default new Controller();
