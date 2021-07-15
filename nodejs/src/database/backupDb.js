import Database from "./Database.js";
import Log from '../Log.js'

const console = new Log('backupDb');


await Database.initDb();

await Database.backup();
console.log("BACKUP DONE");
