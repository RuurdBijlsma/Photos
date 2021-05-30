import {resizeImage} from "../transcode.js";
import path from "path";
import {getExif} from "../exif.js";

// let file = path.resolve('./res/photos/photos/IMG_20210512_032949.jpg');
// let file = path.resolve('./res/photos/photos/IMG_20210512_032955.jpg');
// let file = path.resolve('./res/photos/photos/IMG_20210512_033004.jpg');
let file = path.resolve('./res/photos/photos/IMG_20210512_033016.jpg');


let {exif} = await getExif(file);
console.log(file);
await resizeImage({
    input: file,
    output: `test${exif.Orientation}.webp`,
    height: 700,
    orientation: exif.Orientation,
});
