// import dotenv from 'dotenv';

// dotenv.config();
import {Media} from "../../../database/models/MediaModel.js";
import path from "path";
import config from '../../../config.js'
// import Database from "../../../database/Database.js";
import {getExif, probeVideo} from "../exif.js";

// await Database.initDb();

export async function fixExifs() {
    const startOffset = 0;
    console.log('start offset', startOffset);

    let count = (await Media.count()) - startOffset;
    let batchSize = 8;
    for (let i = 0; i < count; i += batchSize) {
        let items = await Media.findAll({
            limit: batchSize,
            offset: i + startOffset,
        });
        // console.log('offset', i + startOffset, 'limit', batchSize, "items length", items.length)
        let promises = [];
        for (let item of items) {
            promises.push(new Promise(resolve => {
                (async () => {
                    let filePath = path.resolve(path.join(config.media, item.filePath));
                    let metadata;
                    try {
                        if (item.type === 'image') {
                            metadata = await getExif(filePath);
                        } else {
                            metadata = await probeVideo(filePath);
                        }
                    } catch (e) {
                        console.warn("FIX EXIFS ERROR", e);
                    }
                    console.log(item.id, item.filename, metadata.createDate);
                    await item.update({
                        // subType: metadata.subType,
                        // width: metadata.width,
                        // height: metadata.height,
                        // durationMs: metadata.duration,
                        // bytes: metadata.size,
                        createDateString: metadata.createDate,
                        createDate: new Date(metadata.createDate),
                        // exif: metadata.exif,
                    });
                    resolve();
                })();
            }));
        }
        await Promise.all(promises);
        console.log(`Progress [${i + batchSize + startOffset} / ${count}]`);
    }
    console.log("Done fixing");
}
