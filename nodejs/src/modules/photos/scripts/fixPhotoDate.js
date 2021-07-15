import {Media} from "../../../database/models/MediaModel.js";
import {dateFromFile, dateToString, getCreateDate, loadExif} from "../exif.js";
import path from "path";
import config from '../../../config.js'
import Database from "../../../database/Database.js";
import ffmpeg from "../promise-ffmpeg.js";

await Database.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await Media.count()) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await Media.findAll({
        limit: batchSize,
        offset: i + startOffset,
    });
    console.log('offset', i + startOffset, 'limit', batchSize, "items length", items.length)
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
