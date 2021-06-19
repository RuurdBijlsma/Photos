import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import {getCreateDate, loadExif} from "../exif.js";
import path from "path";
import config from '../../../../res/photos/config.json'
import Database from "../../../database/Database.js";

await Database.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await MediaItem.count({where: {type: 'image'}})) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        limit: batchSize,
        offset: i + startOffset,
        where: {type: 'image'},
    });
    let promises = [];
    for (let item of items) {
        promises.push(new Promise(resolve => {
            let filePath = path.resolve(path.join(config.media, item.filePath));
            loadExif(filePath).then(async exif => {
                let createDate = await getCreateDate(filePath, exif);
                if (!isNaN(createDate.getTime())) {
                    await item.update({
                        createDate
                    });
                }
                resolve();
            }).catch(() => {
                console.log(`Can't get exif for ${item.filename}`);
                return resolve();
            });
        }));
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize} / ${count}]`);
}
console.log("Done fixing");
