import {getFileType, text2media} from "./twimote.js";

async function test() {
    await text2media("R I OMEGALUL L U xD 😂");
    await text2media("PepePls hello peepoClap xD")
}

test().then(() => console.log('test complete'));