// import dotenv from 'dotenv';
//
// dotenv.config();

import {Media} from "../../../database/models/MediaModel.js";
import path from "path";
import config from '../../../config.js'
import {getExif, probeVideo} from "../exif.js";

// import Database from "../../../database/Database.js";
//
// await Database.initDb();
// await fixExifs();

export async function fixExifs() {
    const startOffset = 0;
    console.log('start offset', startOffset);

    let count = (await Media.count({where: {type: 'video'}})) - startOffset;
    console.log("db files count", count);
    let batchSize = 10;
    for (let i = 0; i < count; i += batchSize) {
        let items = await Media.findAll({
            where: {type: 'video'},
            limit: batchSize,
            offset: i + startOffset,
        });

        for (let item of items) {
            let filePath = path.resolve(path.join(config.media, item.filePath));
            let metadata;
            try {
                if (item.type === 'image') {
                    metadata = await getExif(filePath);
                } else {
                    metadata = await probeVideo(filePath);
                }
                console.log(item.filename, metadata.createDate);
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
            } catch (e) {
                console.warn("FIX EXIFS ERROR", e);
            }
        }
        console.log(`Progress [${i + batchSize + startOffset} / ${count}]`);
    }
    console.log("Done fixing");
}
