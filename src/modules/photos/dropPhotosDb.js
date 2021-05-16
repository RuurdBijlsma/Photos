import {MediaPlace} from "../../database/models/photos/MediaPlaceModel.js";
import {MediaLocation} from "../../database/models/photos/MediaLocationModel.js";
import {MediaLabel} from "../../database/models/photos/MediaLabelModel.js";
import {MediaGlossary} from "../../database/models/photos/MediaGlossaryModel.js";
import {MediaClassification} from "../../database/models/photos/MediaClassificationModel.js";
import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import cred from "../../../res/auth/credentials.json";
import {Sequelize} from "sequelize";
import Database from "../../database/Database.js";
import {backupDb} from "../../database/models/photos/mediaUtils.js";

const {dbUser, dbPass, dbName} = cred;

const db = new Sequelize(dbName, dbUser, dbPass, {
    host: 'localhost',
    dialect: 'postgres',
    logging: false,
});
await Database.setDb(db);

await backupDb();
console.log("BACKUP DONE");

await MediaItem.drop({cascade: true});
await MediaLocation.drop({cascade: true});
await MediaPlace.drop({cascade: true});
await MediaClassification.drop({cascade: true});
await MediaGlossary.drop({cascade: true});
await MediaLabel.drop({cascade: true});

console.log("DROPPED");
