import {getFileType, text2media} from "./twimote.js";

async function test() {
    await text2media("R I OMEGALUL L U");
    await text2media("PepePls hello peepoClap")
}

test().then(() => console.log('test complete'));