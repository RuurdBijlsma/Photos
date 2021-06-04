import {MediaItem} from "../../../database/models/photos/MediaItemModel.js";
import {
    addSuggestion,
    getClassificationSuggestions,
    getDateSuggestions,
    getPlacesSuggestions
} from "../../../database/models/photos/mediaUtils.js";
import {MediaClassification} from "../../../database/models/photos/MediaClassificationModel.js";
import {MediaLabel} from "../../../database/models/photos/MediaLabelModel.js";
import {MediaLocation} from "../../../database/models/photos/MediaLocationModel.js";
import {MediaPlace} from "../../../database/models/photos/MediaPlaceModel.js";
import Database from "../../../database/Database.js";
import {months} from "../../../utils.js";

await Database.initDb();

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
        let dates = getDateSuggestions(item.createDate);
        let places = getPlacesSuggestions(item.MediaLocation?.MediaPlaces);
        let labels= getClassificationSuggestions(item.MediaClassifications);

        await Database.db.transaction({}, async transaction => {
            await Promise.all([...places, ...labels, ...dates].map(o => addSuggestion(o, transaction)));
        });
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize} / ${count}]`);
}
console.log("Done fixing");
