import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import {dateFromFile, dateToString, getCreateDate, loadExif} from "../exif.js";
import path from "path";
import config from '../../../../res/photos/config.json'
import Database from "../../../database/Database.js";
import ffmpeg from "../promise-ffmpeg.js";

await Database.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await MediaItem.count({where: {createDateString: null}})) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        limit: batchSize,
        offset: i + startOffset,
        where: {createDateString: null},
    });
    console.log("items length", items.length)
    let promises = [];
    for (let item of items) {
        promises.push(new Promise(resolve => {
            (async () => {
                let filePath = path.resolve(path.join(config.media, item.filePath));
                let createDate = null;
                if (item.type === 'image') {
                    let exif = null;
                    try {
                        exif = await loadExif(filePath);
                    } catch (e) {
                        console.log(`Can't get exif for ${item.filename}`);
                    }
                    createDate = await getCreateDate(filePath, exif);
                } else {
                    let {format} = await ffmpeg.ffprobe(filePath);
                    if (format.tags.creation_time)
                        createDate = dateToString(new Date(format.tags.creation_time));
                    if (createDate === null)
                        createDate = dateToString(await dateFromFile(filePath));
                }
                if (createDate) {
                    await item.update({
                        createDateString: createDate,
                        createDate: new Date(createDate),
                    });
                } else {
                    console.log('noooo', filePath)
                }
                resolve();
            })();
        }));
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize + startOffset} / ${count}]`);
}
console.log("Done fixing");
