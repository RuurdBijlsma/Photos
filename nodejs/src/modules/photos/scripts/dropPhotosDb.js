import {Place} from "../../../database/models/PlaceModel.js";
import {Location} from "../../../database/models/LocationModel.js";
import {Label} from "../../../database/models/LabelModel.js";
import {Glossary} from "../../../database/models/GlossaryModel.js";
import {Classification} from "../../../database/models/ClassificationModel.js";
import {Media} from "../../../database/models/MediaModel.js";
import Database from "../../../database/Database.js";

await Database.initDb();

await Database.backup('pre-drop-photos');
console.log("BACKUP DONE");

await Media.drop({cascade: true});
await Location.drop({cascade: true});
await Place.drop({cascade: true});
await Classification.drop({cascade: true});
await Glossary.drop({cascade: true});
await Label.drop({cascade: true});

console.log("DROPPED");
