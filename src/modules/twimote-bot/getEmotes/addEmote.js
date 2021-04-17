import {Emote} from "../../../database/models/EmoteModel.js";
import probe from "probe-image-size";
import fetch from "node-fetch";
import gifyParse from "gify-parse";
import {EmoteSticker} from "../../../database/models/EmoteStickerModel.js";

export default async function addEmote(name, url) {
    let isIn = await Emote.findOne({where: {name}});
    if (isIn !== null) {
        return false;
    }
    let data;
    try {
        data = await probe(url);
    } catch (e) {
        return false;
    }
    let animated = data.type.toLowerCase().includes('gif');
    let duration = 0;
    let frames = 1;
    if (animated) {
        const buffer = await fetch(url).then(f => f.buffer());
        const gifInfo = gifyParse.getInfo(buffer);
        duration = gifInfo.duration;
        frames = gifInfo.images.length;
    }

    let sticker = await EmoteSticker.findOne({where: {text: name}});
    if (sticker !== null)
        await sticker.destroy();
    await Emote.create({
        name,
        ratio: data.width / data.height,
        animated,
        duration,
        frames,
        url,
    });
    return true;
}
