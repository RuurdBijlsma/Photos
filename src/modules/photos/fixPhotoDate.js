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
            exif: {DateTimeOriginal: {[Op.ne]: null}}
        },
        limit: batchSize,
        offset: i,
    });
    let promises = [];
    for (let item of items) {
        let dto = item.exif.DateTimeOriginal;
        if (!dto.includes(' '))
            continue;
        let [date, time] = dto.split(' ');
        date = date.replace(/:/gi, '/');
        item.createDate = new Date(`${date}, ${time}`);
        promises.push(item.save());
    }
    await Promise.all(promises);
    console.log(`Progress [${i + 1} / ${count}]`);
}
console.log("Done fixing");
