import Database from "./Database.js";
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
await Database.restore(file);

