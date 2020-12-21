import Log from "../Log.js";
import {initUser} from "./models/UserModel.js";
import {initSudoku} from "./models/SudokuModel.js";
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

            console.log("Syncing db");
            await this.db.sync();
        } catch (e) {
            console.warn("Postgres connection failed");
        }
    }
}

export default new Database();