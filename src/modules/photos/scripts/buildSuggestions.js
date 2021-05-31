import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import Utils from "../../../Utils.js";
import {addSuggestion} from "../../../database/models/photos/mediaUtils.js";
import {MediaClassification} from "../../../database/models/photos/MediaClassificationModel.js";
import {MediaLabel} from "../../../database/models/photos/MediaLabelModel.js";
import {MediaLocation} from "../../../database/models/photos/MediaLocationModel.js";
import {MediaPlace} from "../../../database/models/photos/MediaPlaceModel.js";

await Utils.initDb();

let count = await MediaItem.count();
let batchSize = 50;
for (let i = 0; i < count; i += batchSize) {
    let items = await MediaItem.findAll({
        where: {
            type: 'image'
        },
        include: [
            {model: MediaClassification, include: [MediaLabel]},
            {model: MediaLocation, include: [MediaPlace]}
        ],
        limit: batchSize,
        offset: i,
    });
    let promises = [];
    for (let item of items) {
        let dates = [];
        if (item.createDate !== null) {
            let date = item.createDate;
            let month = Utils.months[date.getMonth()];
            let year = date.getFullYear().toString();
            dates.push({type: 'date', text: month});
            dates.push({type: 'date', text: year});
            dates.push({type: 'date', text: `${month} ${year}`});
        }

        let places = (item.MediaLocation?.MediaPlaces?.map?.(p => p?.text) ?? [])
            .map(p => ({type: 'place', text: p}));
        let labels = (item.MediaClassifications?.flatMap?.(c => c?.MediaLabels.map(l => l.text)) ?? [])
            .map(p => ({type: 'label', text: p}));

        await Promise.all([...places, ...labels, ...dates].map(addSuggestion));
    }
    await Promise.all(promises);
    console.log(`Progress [${i + 1} / ${count}]`);
}
console.log("Done fixing");
