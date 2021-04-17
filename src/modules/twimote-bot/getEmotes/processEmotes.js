import rawEmotes from "./rawEmotes.js";
import defaultEmotes from "./defaultEmotes.js";
import {Emote} from "../../../database/models/EmoteModel.js";
import Database from "../../../database/Database.js";
import cred from "../../../../res/auth/credentials.json"
import {Sequelize} from "sequelize";
import addEmote from "./addEmote.js";

(async () => {
    const {dbUser, dbPass, dbName} = cred;

    let allEmotes = defaultEmotes.concat(rawEmotes);
    let fast = process.argv[process.argv.length - 1] === 'fast';
    let i = 0;

    const db = new Sequelize(dbName, dbUser, dbPass, {
        host: 'localhost',
        dialect: 'postgres',
        logging: false,
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
        if (await addEmote(name, url)) {
            console.log(`[${i}/${allEmotes.length}] Processed ${name}`);
        } else {
            console.log(`[${i}/${allEmotes.length}] Skipping ${name}, already included`);
        }
    }
})();
