import {Media} from "../../../database/models/MediaModel.js";
import {imageSize, loadExif} from "../exif.js";
import path from "path";
import config from '../../../config.js'
import Database from "../../../database/Database.js";

await Database.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await Media.count({where: {type: 'image'}})) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await Media.findAll({
        limit: batchSize,
        offset: i + startOffset,
        where: {type: 'image'},
    });
    let promises = [];
    for (let item of items) {
        promises.push(new Promise(async resolve => {
            // if (!item.filename.includes("edited"))
            //     return resolve();
            let filePath = path.resolve(path.join(config.media, item.filePath));
            let exif = null;
            try {
                exif = await loadExif(filePath);
            } catch (e) {
                console.log(`Can't get exif for ${item.filename}`);
            }
            let imgDim = await imageSize(filePath, exif);

            if (imgDim !== null && isFinite(imgDim.height) && isFinite(imgDim.width)) {
                await item.update({
                    width: imgDim.width,
                    height: imgDim.height,
                });
            }
            resolve();
        }));
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize} / ${count}]`);
}
console.log("Done fixing");
