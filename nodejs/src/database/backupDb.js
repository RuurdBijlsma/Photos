import Database from "./Database.js";

await Database.initDb();

await Database.backup();
console.log("BACKUP DONE");
