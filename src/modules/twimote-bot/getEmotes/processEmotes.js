import rawEmotes from "./rawEmotes.js";
import probe from "probe-image-size";
import fs from "fs";
import fetch from 'node-fetch';
import path from "path";
import defaultEmotes from "./defaultEmotes.js";
import gifyParse from "gify-parse";
import emotes from "../emotes.js";

let allEmotes = defaultEmotes.concat(rawEmotes);
let fast = process.argv[process.argv.length - 1] === 'fast';
let result = fast ? {...emotes} : {}, i = 0;

for (let {name, url} of allEmotes) {
    i++;
    if (result.hasOwnProperty(name)) {
        console.log(`[${i}/${allEmotes.length}] Skipping ${name}, already included`);
        continue;
    }
    if (fast && emotes.hasOwnProperty(name)) {
        console.log(`[${i}/${allEmotes.length}] Skipping ${name}, is already in emotes.js and FAST mode is enabled`);
        continue;
    }
    let data = await probe(url);
    let animated = data.type.toLowerCase().includes('gif');
    let duration = 0;
    let frames = 1;
    if (animated) {
        const buffer = await fetch(url).then(f => f.buffer());
        const gifInfo = gifyParse.getInfo(buffer);
        duration = gifInfo.duration;
        frames = gifInfo.images.length;
    }
    result[name] = {
        ratio: data.width / data.height,
        animated,
        duration,
        frames,
        url: url,
    }
    console.log(`[${i}/${allEmotes.length}] Processed ${name}`);
}

let emotesFile = path.resolve('src/modules/twimote-bot/emotes.js');
console.log(`Writing to ${emotesFile}`)
fs.writeFileSync(emotesFile, 'export default ' + JSON.stringify(result));