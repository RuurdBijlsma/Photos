import {MediaItem} from "../../database/models/photos/MediaItemModel.js";
import cred from "../../../res/auth/credentials.json";
import {Sequelize} from "sequelize";
import Database from "../../database/Database.js";

const {Op} = Sequelize;
const {dbUser, dbPass, dbName} = cred;

const db = new Sequelize(dbName, dbUser, dbPass, {
    host: 'localhost',
    dialect: 'postgres',
    logging: false,
});
await Database.setDb(db);

let count = await MediaItem.count();
let batchSize = 50;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        where: {
            type: 'image'
        },
        limit: batchSize,
        offset: i,
    });
    let promises = [];
    for (let item of items) {
        let subType = 'none';
        if (item.filename.includes("PORTRAIT") && item.filename.includes("COVER"))
            subType = 'Portrait';
        else if (item.filename.startsWith('PANO'))
            subType = 'VR';
        if (subType !== item.subType) {
            console.log(`Fixing subtype of ${item.filename} from ${item.subType} to ${subType}`)
        }
        item.subType = subType;
        promises.push(item.save());
    }
    await Promise.all(promises);
    console.log(`Progress [${i + 1} / ${count}]`);
}
console.log("Done fixing");
