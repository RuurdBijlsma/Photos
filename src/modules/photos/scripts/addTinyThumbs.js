import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import Utils from "../../../Utils.js";
import {resizeImage} from "../transcode.js";
import {getPaths} from '../watchAndSynchonize.js'
import path from "path";
import config from '../../../../res/photos/config.json';

await Utils.initDb();

let count = await MediaItem.count();
let batchSize = 50;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        limit: batchSize,
        offset: i,
    });
    let promises = [];
    for (let item of items) {
        let tinyHeight = Math.min(item.height, 260);
        let {tiny} = getPaths(item.id);
        let inputImg = path.resolve(item.type === 'video' ?
            path.join(config.thumbnails, 'big', `${item.id}.webp`) :
            path.join(config.media, item.filePath));
        promises.push(
            resizeImage({input: inputImg, output: tiny, height: tinyHeight,})
        );
    }
    await Promise.all(promises);
    console.log(`Progress [${i + 1} / ${count}]`);
}
console.log("Done creating tiny images");
