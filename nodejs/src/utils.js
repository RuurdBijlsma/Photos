import crypto from "crypto";
import fs from "fs";
import os from "os";

export const freeGb = os.freemem() / 1000000000;
export const batchSize = Math.min(Math.max(1, Math.ceil(freeGb)), 30);
export const months = ['January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'];
export const shortMonths = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];


export function isDate(query) {
    const lowerQuery = query.toLowerCase();
    let lowerMonths = [...months, ...shortMonths].map(m => m.toLowerCase());
    if (lowerMonths.includes(lowerQuery))
        return {type: 'month', month: (lowerMonths.indexOf(lowerQuery) % 12) + 1};

    let parts = lowerQuery.split(/[- _\/]/).filter(n => n.length !== 0);
    if (parts.length !== 2)
        return {type: 'none'};

    // Format is now either [6 jan] [jan 6] [6 1] [24 6]
    let writtenMonthFirst = lowerMonths.includes(parts[0]);
    let writtenMonthSecond = lowerMonths.includes(parts[1]);
    let highNumberSecond = (+parts[1] > 12 && +parts[0] <= 12);
    if (writtenMonthFirst && +parts[1] <= 31 && +parts[1] >= 0) {
        // jan 6
        return {type: 'dayMonth', month: (lowerMonths.indexOf(parts[0]) % 12) + 1, day: +parts[1]};
    } else if (writtenMonthSecond && +parts[0] <= 31 && +parts[0] >= 0) {
        // 6 jan
        return {type: 'dayMonth', month: (lowerMonths.indexOf(parts[1]) % 12) + 1, day: +parts[0]};
    } else if (highNumberSecond && +parts[0] <= 12 && +parts[0] >= 0 && +parts[1] <= 31 && +parts[1] >= 0) {
        // 6 18
        return {type: 'dayMonth', month: +parts[0], day: +parts[1]};
    } else if (+parts[0] <= 31 && +parts[0] >= 0 && +parts[1] <= 12 && +parts[1] >= 0) {
        // 26 1
        return {type: 'dayMonth', month: +parts[1], day: +parts[0]};
    }

    return {type: 'none'};
}

export async function checkFileExists(file) {
    return fs.promises.access(file, fs.constants.F_OK)
        .then(() => true)
        .catch(() => false);
}


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
