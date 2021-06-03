import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import Utils from "../../../Utils.js";
import {imageSize, loadExif} from "../exif.js";
import path from "path";
import config from '../../../../res/photos/config.json'

await Utils.initDb();

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
        promises.push(new Promise(async resolve => {
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
