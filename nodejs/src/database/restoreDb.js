import Database from "./Database.js";
import {User} from "./models/UserModel.js";
import {MediaSuggestion} from "./models/MediaSuggestionModel.js";
import {MediaLocation} from "./models/MediaLocationModel.js";
import {MediaClassification} from "./models/MediaClassificationModel.js";
import {MediaLabel} from "./models/MediaLabelModel.js";
import {MediaPlace} from "./models/MediaPlaceModel.js";
import {MediaGlossary} from "./models/MediaGlossaryModel.js";
import {MediaItem} from "./models/MediaItemModel.js";
import path from "path";
import {checkFileExists} from "../utils.js";

const restoreFile = process.argv[2];
if (!restoreFile)
    throw new Error("Can't restore, no file given");
let file = path.resolve(restoreFile);
if (!await checkFileExists(file))
    throw new Error(`File not found: ${file}`);

await Database.initDb();

console.log("Backing up before restoring");
await Database.backup('pre-restore');
await MediaItem.drop({cascade: true});
await MediaGlossary.drop({cascade: true});
await MediaPlace.drop({cascade: true});
await MediaLabel.drop({cascade: true});
await MediaClassification.drop({cascade: true});
await MediaLocation.drop({cascade: true});
await MediaSuggestion.drop({cascade: true});
await User.drop({cascade: true});
console.log("Dropped all tables before restoring");
await Database.restore(file);

