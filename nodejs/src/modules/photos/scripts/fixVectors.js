import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import Utils from "../../../Utils.js";
import {dateToWords, toVector} from "../../../database/models/photos/mediaUtils.js";
import {MediaClassification} from "../../../database/models/photos/MediaClassificationModel.js";
import {MediaLabel} from "../../../database/models/photos/MediaLabelModel.js";
import {MediaLocation} from "../../../database/models/photos/MediaLocationModel.js";
import {MediaPlace} from "../../../database/models/photos/MediaPlaceModel.js";
import Database from "../../../database/Database.js";
import {MediaGlossary} from "../../../database/models/photos/MediaGlossaryModel.js";

await Utils.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await MediaItem.count()) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        include: [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation, include: [MediaPlace]}
        ],
        limit: batchSize,
        offset: i + startOffset,
    });
    let promises = [];
    for (let item of items) {
        let aWords = [], bWords = [], cWords = [];

        let places = item.MediaLocation?.MediaPlaces;
        if (Array.isArray(places)) {
            let place = places.find(p => p.type === 'place')?.text;
            let country = places.find(p => p.type === 'country')?.text;
            let admin1 = places.find(p => p.type === 'admin1')?.text;
            let admin2 = places.find(p => p.type === 'admin2')?.text;
            let admin3 = places.find(p => p.type === 'admin3')?.text;
            let admin4 = places.find(p => p.type === 'admin4')?.text;
            aWords.push(place, country);
            bWords.push(admin1, admin2, admin3, admin4);
        }

        let classes = item.MediaClassifications.map(c => ({
            labels: c.MediaLabels.map(l => l.text),
            glossaries: c.MediaGlossaries.map(l => l.text),
        }));
        if (classes[0]) {
            aWords.push(...(classes[0]?.labels ?? []))
            cWords.push(...(classes[0]?.glossaries ?? []))
        }
        if (classes[1]) {
            bWords.push(...(classes[1]?.labels ?? []))
            cWords.push(...(classes[1]?.glossaries ?? []))
        }
        if (classes[2]) {
            cWords.push(...(classes[2]?.labels ?? []))
            cWords.push(...(classes[2]?.glossaries ?? []))
        }

        aWords.push(...dateToWords(item.createDate))

        bWords.push(item.filename);
        cWords.push(item.type);
        if (item.subType !== 'none')
            bWords.push(item.subType);

        aWords = aWords.filter(w => w !== null && w !== undefined).map(w => w.toString());
        bWords = bWords.filter(w => w !== null && w !== undefined).map(w => w.toString());
        cWords = cWords.filter(w => w !== null && w !== undefined).map(w => w.toString());

        const p = new Promise(async resolve => {
            await Database.db.transaction({}, async transaction => {
                try {
                    // console.log("Saving item", item.filename);
                    await item.update({
                        vectorA: toVector('A', ...aWords),
                        vectorB: toVector('B', ...bWords),
                        vectorC: toVector('C', ...cWords),
                    }, {transaction})
                    await Database.db.query(
                        `update "MediaItems"
                        set vector = "vectorA" || "vectorB" || "vectorC"
                        where id = '${item.id}'`
                        , {transaction});
                } catch (e) {
                    console.warn(e);
                } finally {
                    console.log("TRANSACTION COMPLETE", item.filename);
                }
            });
            resolve();
        })

        promises.push(p);
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize} / ${count}]`);
}
console.log("Done fixing");