import rawEmotes from "./rawEmotes.js";
import defaultEmotes from "./defaultEmotes.js";
import probe from "probe-image-size";
import fetch from 'node-fetch';
import gifyParse from "gify-parse";
import {Emote} from "../../../database/models/EmoteModel.js";
import Database from "../../../database/Database.js";
import cred from "../../../../res/auth/credentials.json"
import {Sequelize} from "sequelize";

(async () => {
    const {dbUser, dbPass, dbName} = cred;

    let allEmotes = defaultEmotes.concat(rawEmotes);
    let fast = process.argv[process.argv.length - 1] === 'fast';
    let i = 0;

    const db = new Sequelize(dbName, dbUser, dbPass, {
        host: 'localhost',
        dialect: 'postgres',
    });
    await Database.setDb(db);

    if (!fast) {
        console.log("DROPPING EMOTE TABLE");
        try {
            await Emote.drop();
            await Emote.sync();
        } catch (e) {
            console.log("EMOTE drop error", e);
        }
    }

    for (let {name, url} of allEmotes) {
        i++;
        let isIn = await Emote.findOne({where: {name}});
        if (isIn !== null) {
            console.log(`[${i}/${allEmotes.length}] Skipping ${name}, already included`);
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
        await Emote.create({
            name,
            ratio: data.width / data.height,
            animated,
            duration,
            frames,
            url,
        });
        console.log(`[${i}/${allEmotes.length}] Processed ${name}`);
    }
})();