import {Worker, isMainThread, parentPort, workerData} from 'worker_threads';
import youtube from "./Youtube.mjs";
import Log from "../Log.mjs";

let filePath = workerData.destinationPath;
let id = workerData.id;

youtube.download(id, filePath).then(() => {
    parentPort.postMessage("Completed");
});