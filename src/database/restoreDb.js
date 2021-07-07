import Database from "./Database.js";
import {EmoteSticker} from "./models/EmoteStickerModel.js";
import {Emote} from "./models/EmoteModel.js";
import {Sudoku} from "./models/SudokuModel.js";
import {User} from "./models/UserModel.js";
import {MediaSuggestion} from "./models/photos/MediaSuggestionModel.js";
import {MediaLocation} from "./models/photos/MediaLocationModel.js";
import {MediaClassification} from "./models/photos/MediaClassificationModel.js";
import {MediaLabel} from "./models/photos/MediaLabelModel.js";
import {MediaPlace} from "./models/photos/MediaPlaceModel.js";
import {MediaGlossary} from "./models/photos/MediaGlossaryModel.js";
import {MediaItem} from "./models/photos/MediaItemModel.js";
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
await Sudoku.drop({cascade: true});
await Emote.drop({cascade: true});
await EmoteSticker.drop({cascade: true});
console.log("Dropped all tables before restoring");
await Database.restore(file);

