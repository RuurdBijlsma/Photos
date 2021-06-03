import Log from "../Log.js";
import {initUser} from "./models/UserModel.js";
import {initSudoku} from "./models/SudokuModel.js";
import {initEmoteSticker} from "./models/EmoteStickerModel.js";
import {initEmote} from "./models/EmoteModel.js";
import {initMedia} from "./models/photos/mediaUtils.js";
import path from "path";
import {exec} from "child_process";
import {checkFileExists} from "../modules/photos/watchAndSynchonize";
import {Sequelize} from "sequelize";
import cred from "../../res/auth/credentials.json";

const {dbUser, dbPass, dbName, dbHost, dbPort} = cred;
const console = new Log("Database");

class Database {
    constructor() {
        this.db = null;
    }

    async setDb(db) {
        this.db = db;
        await this.init();
    }

    async init() {
        try {
            await this.db.authenticate();
            console.log("Postgres connection started successfully");

            console.log("Init user");
            initUser(this.db);
            console.log("Init sudoku");
            initSudoku(this.db);
            console.log("Init emote sticker");
            initEmoteSticker(this.db);
            console.log("Init emote");
            initEmote(this.db);
            console.log("Init media tables");
            await initMedia(this.db);

            console.log("Syncing db");
            await this.db.sync();

            // await insertTestMediaItem();
            // await removeTestMediaItem();
        } catch (e) {
            console.warn("Postgres connection failed", e);
        }
    }

    async backup(name = '') {
        return new Promise((resolve, reject) => {
            if (name !== '')
                name += '_';
            let dateTime = new Date().toJSON().replace(/:/g, '_');
            let backupTo = path.resolve(`./res/photos/backups/rsdb_${name}${dateTime}.dump`);
            console.log(`Backing up database! ${backupTo}`);
            exec(`pg_dump --dbname=${this.connectionString} -Fc > ${backupTo}`,
                (error, stderr, stdout) => {
                    if (error) {
                        console.warn('db backup error', error);
                        return reject(error);
                    }
                }).on('close', resolve);
        });
    }

    /**
     * @param filePath:string
     * @returns {Promise<boolean>}
     */
    async restore(filePath) {
        if (!filePath)
            return false;
        let file = path.resolve(filePath);
        if (!await checkFileExists(file))
            return false;
        return new Promise(resolve => {
            console.log(`Restoring database! ${file}`);
            exec(`pg_restore.exe --dbname=${this.connectionString} ${file}`,
                (error, stderr, stdout) => {
                    if (error) {
                        console.warn('db restore error', error);
                        return resolve(false);
                    }
                }).on('close', () => resolve(true));
        });
    }

    get connectionString() {
        return `postgresql://${dbUser}:${dbPass}@${dbHost}:${dbPort}/${dbName}`;
    }

    async initDb() {
        const db = new Sequelize(dbName, dbUser, dbPass, {
            host: cred.dbHost,
            dialect: 'postgres',
            logging: false,
            pool: {
                acquire: 20000,
            }
        });
        await this.setDb(db);
        return db;
    }
}

export default new Database();
