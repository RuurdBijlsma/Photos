import rawEmotes from "./rawEmotes.js";
import probe from "probe-image-size";
import fs from "fs";
import path from "path";
import defaultEmotes from "./defaultEmotes.js";

let emotes = rawEmotes.concat(defaultEmotes);
let result = {}, i = 0;
for (let {name, url} of emotes) {
    i++;
    if (result.hasOwnProperty(name)) {
        console.log(`[${i}/${emotes.length}] Skipping ${name}, already included`);
        continue;
    }
    let data = await probe(url);
    result[name] = {
        ratio: data.width / data.height,
        animated: data.type.toLowerCase().includes('gif'),
        url: url,
    }
    console.log(`[${i}/${emotes.length}] Processed ${name}`);
}

let emotesFile = path.resolve('src/modules/twimote-bot/emotes.js');
console.log(`Writing to ${emotesFile}`)
fs.writeFileSync(emotesFile, 'export default ' + JSON.stringify(result));