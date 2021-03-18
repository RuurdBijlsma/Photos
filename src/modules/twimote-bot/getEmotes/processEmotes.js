import rawEmotes from "./rawEmotes.js";
import probe from "probe-image-size";
import fs from "fs";
import fetch from 'node-fetch';
import path from "path";
import defaultEmotes from "./defaultEmotes.js";
import gifyParse from "gify-parse";

let emotes = rawEmotes.concat(defaultEmotes);
let result = {}, i = 0;
for (let {name, url} of emotes) {
    i++;
    if (result.hasOwnProperty(name)) {
        console.log(`[${i}/${emotes.length}] Skipping ${name}, already included`);
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
    console.log(`[${i}/${emotes.length}] Processed ${name}`);
}

let emotesFile = path.resolve('src/modules/twimote-bot/emotes.js');
console.log(`Writing to ${emotesFile}`)
fs.writeFileSync(emotesFile, 'export default ' + JSON.stringify(result));