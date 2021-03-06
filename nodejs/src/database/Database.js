import {initUser} from "./models/UserModel.js";
import {initTables} from "./models/mediaUtils.js";
import path from "path";
import {exec} from "child_process";
import {Sequelize} from "sequelize";
import {checkFileExists} from "../utils.js";
import config from "../config.js";
import Clog from "../Clog.js";
import DbInfo from "./DbInfo.js";
import {LogSession} from "./models/LogSessionModel.js";
import dotenv from 'dotenv';

dotenv.config();
const console = new Clog("Database");

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
            console.log("Init media tables");
            await initTables(this.db);

            console.log("Syncing db");
            await this.db.sync();
            let session = await LogSession.create();
            DbInfo.session = session.id;
            DbInfo.isConnected = true;
        } catch (e) {
            console.warn("Postgres connection failed", e);
        }
    }

    async backup(name = '') {
        return new Promise((resolve, reject) => {
            if (name !== '')
                name = '_' + name;
            let dateTime = new Date().toJSON().replace(/:/g, '_');
            let backupTo = path.resolve(path.join(config.backups, `${dateTime}_rpdb${name}.dump`));
            const backupCommand = `pg_dump --dbname=${this.connectionString} -Fc --clean --create --if-exists > ${backupTo}`;
            console.log(`Backing up database! ${backupTo}`);
            exec(backupCommand,
                (error, stderr, stdout) => {
                    if (error) {
                        console.warn('db backup error', error);
                        return reject(error);
                    }
                }).on('close',
                () => {
                    console.log("Backup complete");
                    resolve();
                });
        });
    }

    /**
     * @param filePath:string
     * @param dropAll:boolean
     * @returns {Promise<boolean>}
     */
    async restore(filePath, dropAll = true) {
        if (!filePath)
            return false;
        let file = path.resolve(filePath);
        if (!await checkFileExists(file))
            return false;

        // if (dropAll) {
        //     console.log("Backing up before restoring");
        //     await this.backup('pre-restore');
        //     await Media.drop({cascade: true});
        //     await Glossary.drop({cascade: true});
        //     await Place.drop({cascade: true});
        //     await Label.drop({cascade: true});
        //     await Classification.drop({cascade: true});
        //     await Location.drop({cascade: true});
        //     await Suggestion.drop({cascade: true});
        //     await User.drop({cascade: true});
        //     console.log("Dropped all tables before restoring");
        //     await this.db.sync();
        // }

        return new Promise(resolve => {
            console.log(`Restoring database! ${file}`);
            console.warn("==========================================================");
            console.warn("=====PROGRAM WILL RESTART AFTER RESTORE IS COMPLETE!======");
            console.warn("====IF THE PROGRAM DOES NOT RESTART START IT MANUALLY.====");
            console.warn("==========================================================");
            setTimeout(() => exec(`pg_restore --clean --create --dbname=${this.connectionString} ${file}`,
                (error, stderr, stdout) => {
                    if (error) {
                        console.warn('db restore error', error);
                        console.log("Restore complete");
                        return resolve(false);
                    }
                }).on('close',
                () => {
                    console.log("Restore complete");
                    process.exit(0);
                    resolve(true);
                }), 1000);
        });
    }

    get connectionString() {
        return `postgresql://${this.dbConfig.user}:${this.dbConfig.pass}@${this.dbConfig.host}:${this.dbConfig.port}/${this.dbConfig.schema}`;
    }

    get dbConfig() {
        return {
            user: process.env.DB_USER ?? 'postgres',
            pass: process.env.DB_PASSWORD ?? 'postgres',
            host: process.env.DB_HOST ?? 'localhost',
            port: process.env.DB_PORT ?? 5432,
            schema: process.env.DB_SCHEMA ?? 'postgres',
            ssl: process.env.DB_SSL === "true",
        }
    }

    async initDb() {
        const db = new Sequelize(this.dbConfig.schema,
            this.dbConfig.user,
            this.dbConfig.pass,
            {
                host: this.dbConfig.host,
                port: this.dbConfig.port,
                dialect: 'postgres',
                logging: false,
                dialectOptions: {
                    ssl: this.dbConfig.ssl,
                },
                pool: {
                    acquire: 20000,
                },
            });
        await this.setDb(db);
        return db;
    }
}

export default new Database();
