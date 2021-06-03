import {backupDb} from "../../../database/models/photos/mediaUtils.js";
import Utils from "../../../Utils.js";

await Utils.initDb();

await backupDb();
console.log("BACKUP DONE");
