import Database from "./Database.js";
import Clog from '../Clog.js'

const console = new Clog('backupDb');


await Database.initDb();

await Database.backup();
console.log("BACKUP DONE");
