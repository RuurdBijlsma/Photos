import {Media} from "../../../database/models/MediaModel.js";
import {
    addSuggestion,
    getClassificationSuggestions,
    getDateSuggestions,
    getPlacesSuggestions
} from "../../../database/models/mediaUtils.js";
import {Classification} from "../../../database/models/ClassificationModel.js";
import {Label} from "../../../database/models/LabelModel.js";
import {Location} from "../../../database/models/LocationModel.js";
import {Place} from "../../../database/models/PlaceModel.js";
import Database from "../../../database/Database.js";

await Database.initDb();

let count = await Media.count();
let batchSize = 50;
for (let i = 0; i < count; i += batchSize) {
    let items = await Media.findAll({
        where: {
            type: 'image'
        },
        include: [
            {model: Classification, include: [Label]},
            {model: Location, include: [Place]}
        ],
        limit: batchSize,
        offset: i,
    });
    let promises = [];
    for (let item of items) {
        let dates = getDateSuggestions(item.createDate);
        let places = getPlacesSuggestions(item.Location?.Places);
        let labels= getClassificationSuggestions(item.Classifications);

        await Database.db.transaction({}, async transaction => {
            await Promise.all([...places, ...labels, ...dates].map(o => addSuggestion(o, transaction)));
        });
    }
    await Promise.all(promises);
    console.log(`Progress [${i + batchSize} / ${count}]`);
}
console.log("Done fixing");
