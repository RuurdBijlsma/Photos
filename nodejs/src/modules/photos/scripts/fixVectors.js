import {Media} from "../../../database/models/MediaModel.js";
import {dateToWords, toVector} from "../../../database/models/mediaUtils.js";
import {Classification} from "../../../database/models/ClassificationModel.js";
import {Label} from "../../../database/models/LabelModel.js";
import {Location} from "../../../database/models/LocationModel.js";
import {Place} from "../../../database/models/PlaceModel.js";
import Database from "../../../database/Database.js";
import {Glossary} from "../../../database/models/GlossaryModel.js";

await Database.initDb();

const startOffset = +(process.argv[2] ?? 0);
console.log('start offset', startOffset);

let count = (await Media.count()) - startOffset;
let batchSize = 20;
for (let i = 0; i < count; i += batchSize) {
    let items = await Media.findAll({
        include: [
            {model: Classification, include: [Label, Glossary]},
            {model: Location, include: [Place]}
        ],
        limit: batchSize,
        offset: i + startOffset,
    });
    let promises = [];
    for (let item of items) {
        let aWords = [], bWords = [], cWords = [];

        let places = item.Location?.Places;
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

        let classes = item.Classifications.map(c => ({
            labels: c.Labels.map(l => l.text),
            glossaries: c.Glossaries.map(l => l.text),
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
                        `update "Media"
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
