import {Media} from "../../../database/models/MediaModel.js";
import Database from "../../../database/Database.js";


await Database.initDb();

let count = await Media.count();
let batchSize = 50;
for (let i = 0; i < count; i += batchSize) {
    let items = await Media.findAll({
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
