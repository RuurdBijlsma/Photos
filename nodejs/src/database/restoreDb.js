import Database from "./Database.js";
import {User} from "./models/UserModel.js";
import {Suggestion} from "./models/SuggestionModel.js";
import {Location} from "./models/LocationModel.js";
import {Classification} from "./models/ClassificationModel.js";
import {Label} from "./models/LabelModel.js";
import {Place} from "./models/PlaceModel.js";
import {Glossary} from "./models/GlossaryModel.js";
import {Media} from "./models/MediaModel.js";
import path from "path";
import {checkFileExists} from "../utils.js";
import Clog from '../Clog.js'

const console = new Clog('restoreDb');

const restoreFile = process.argv[2];
if (!restoreFile)
    throw new Error("Can't restore, no file given");
let file = path.resolve(restoreFile);
if (!await checkFileExists(file))
    throw new Error(`File not found: ${file}`);

await Database.initDb();

console.log("Backing up before restoring");
await Database.backup('pre-restore');
await Media.drop({cascade: true});
await Glossary.drop({cascade: true});
await Place.drop({cascade: true});
await Label.drop({cascade: true});
await Classification.drop({cascade: true});
await Location.drop({cascade: true});
await Suggestion.drop({cascade: true});
await User.drop({cascade: true});
console.log("Dropped all tables before restoring");
await Database.restore(file);

