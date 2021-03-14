import Log from "./Log.js";
import crypto from "crypto";
const console = new Log("Utils");

export default class Utils {
    static getToken() {
        return new Promise((resolve, reject) => {
            crypto.randomBytes(48, (err, buffer) => {
                if (err) {
                    reject(err);
                    return;
                }

                resolve(buffer.toString('hex'));
            });
        });
    }

    static bytesToReadable(bytes) {
        let length = Math.log10(bytes);
        if (length < 2) {
            return bytes + ' B';
        } else if (length < 5) {
            return (bytes / 1024).toFixed(2) + ' kB';
        } else if (length < 8) {
            return (bytes / (1024 ** 2)).toFixed(2) + ' MB';
        } else if (length < 12) {
            return (bytes / (1024 ** 3)).toFixed(2) + ' GB';
        } else if (length < 15) {
            return (bytes / (1024 ** 4)).toFixed(2) + ' TB';
        }
        return 'very bige bytes';
    }
}