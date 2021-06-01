import Log from "./Log.js";
import crypto from "crypto";
import {Sequelize} from "sequelize";
import cred from "../res/auth/credentials.json";
import Database from "./database/Database.js";

const console = new Log("Utils");
const {dbUser, dbPass, dbName} = cred;


export default class Utils {
    static get months() {
        return ['January', 'February', 'March', 'April', 'May', 'June',
            'July', 'August', 'September', 'October', 'November', 'December'];
    }

    static async initDb() {
        const db = new Sequelize(dbName, dbUser, dbPass, {
            host: 'localhost',
            dialect: 'postgres',
            logging: false,
            pool: {
                acquire: 20000,
            }
        });
        await Database.setDb(db);
        return db;
    }

    static getToken(nBytes = 48) {
        return new Promise((resolve, reject) => {
            crypto.randomBytes(nBytes, (err, buffer) => {
                if (err) {
                    reject(err);
                    return;
                }

                resolve(buffer.toString('hex'));
            });
        });
    }

    static bytesToReadable(bytes) {
        let length = Math.log10(bytes);
        if (length < 2) {
            return bytes + ' B';
        } else if (length < 5) {
            return (bytes / 1024).toFixed(2) + ' kB';
        } else if (length < 8) {
            return (bytes / (1024 ** 2)).toFixed(2) + ' MB';
        } else if (length < 12) {
            return (bytes / (1024 ** 3)).toFixed(2) + ' GB';
        } else if (length < 15) {
            return (bytes / (1024 ** 4)).toFixed(2) + ' TB';
        }
        return 'very bige bytes';
    }
}
