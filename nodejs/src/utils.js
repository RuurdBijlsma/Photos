import crypto from "crypto";
import fs from "fs";
import os from "os";

export const freeGb = os.freemem() / 1000000000;
export const batchSize = Math.min(Math.max(1, Math.ceil(freeGb * 3)), 30);

export async function checkFileExists(file) {
    return fs.promises.access(file, fs.constants.F_OK)
        .then(() => true)
        .catch(() => false);
}

export const months = ['January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'];

export function getToken(nBytes = 48) {
    return new Promise((resolve, reject) => {
        crypto.randomBytes(nBytes, (err, buffer) => {
            if (err) {
                reject(err);
                return;
            }

            resolve(buffer.toString('hex'));
        });
    });
}

export function bytesToReadable(bytes) {
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
