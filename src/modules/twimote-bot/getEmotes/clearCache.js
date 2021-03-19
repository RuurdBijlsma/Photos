import Database from "../../../database/Database.js";
import cred from "../../../../res/auth/credentials.json"
import {Sequelize} from "sequelize";
import fs from 'fs';
import path from 'path';
import {EmoteSticker} from "../../../database/models/EmoteStickerModel.js";

(async () => {
    await dropStickersTable();
    console.log(`\n\nDropped emote stickers table`);
    let emotes = path.resolve('./res/twimote/emotes');
    await clearFolder(emotes)
    console.log(`Cleared folder ${emotes}`);

    let cache = path.resolve('./res/twimote/cache');
    await clearFolder(cache)
    console.log(`Cleared folder ${cache}`);
})();

async function clearFolder(folder) {
    try {
        let files = await fs.promises.readdir(folder);
        for (const file of files) {
            if (file === 'OMEGALUL.png') continue;
            await fs.promises.unlink(path.join(folder, file));
        }
    } catch (e) {
        console.log(`Clear folder ${folder} error`, e);
    }
}

async function dropStickersTable() {
    const {dbUser, dbPass, dbName} = cred;

    const db = new Sequelize(dbName, dbUser, dbPass, {
        host: 'localhost',
        dialect: 'postgres',
    });
    await Database.setDb(db);
    try {
        console.log("DROPPING EMOTE STICKERS TABLE");
        await EmoteSticker.drop();
        await EmoteSticker.sync();
    } catch (e) {
        console.log("EMOTE STICKERS drop error", e);
    }
}