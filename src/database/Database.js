import Log from "../Log.js";
import {initUser} from "./models/UserModel.js";
import {initSudoku} from "./models/SudokuModel.js";

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
            Log.l('Auth', "Postgres connection started successfully");

            initUser(this.db);
            initSudoku(this.db);

            await this.db.sync();
        } catch (e) {
            Log.w('Auth', "Postgres connection started successfully");
        }
    }
}

export default new Database();