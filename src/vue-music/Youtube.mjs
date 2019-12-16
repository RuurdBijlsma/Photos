import ytdl from "ytdl-core";
import fs from "fs";
import secrets from "../../res/vue-music/secrets.json";
import youtubeSearch from "youtube-search";
import Log from "../Log.mjs";

class Youtube {
    constructor() {
        this.baseUrl = 'http://www.youtube.com/watch?v=';
        this.ytdlOptions = {
            quality: 'highestaudio',
            filter: 'audioonly',
        };
        this.searchCache = {};
    }

    urlById(id) {
        return this.baseUrl + id;
    }

    async download(id, destinationFile) {
        return new Promise(resolve => {
            let stream = ytdl(this.urlById(id), this.ytdlOptions);
            Log.l("Youtube", "Download STARTED ", destinationFile);

            stream.on('progress', (chunkLength, downloaded, totalLength) => {
                if (downloaded === totalLength) {
                    Log.l("Youtube", "Download FINISHED ", destinationFile);
                    fs.rename(destinationFile + '.temp', destinationFile, err => {
                        if (err) Log.w("Youtube", "Could not rename destination file", destinationFile);
                        resolve(downloaded);
                    });
                }
            });

            stream.pipe(fs.createWriteStream(destinationFile + '.temp'));
        })
    }

    async getSongByteLength(songUrl, options) {
        return new Promise((resolve, reject) => {

            let stream = ytdl(songUrl, options);
            stream.on('response', httpResponse => {
                resolve(parseInt(httpResponse.headers["content-length"]));
                stream.destroy();
            });
            stream.on('error', error => {
                reject(error);
                stream.destroy();
            });
        })

    }

    timeout(ms, promise) {
        return new Promise(function (resolve, reject) {
            setTimeout(function () {
                reject(new Error("timeout"))
            }, ms);
            promise.then(resolve, reject);
        })
    }

    async stream(req, res, id) {
        let fileSize;
        try {
            fileSize = await this.timeout(5000,
                this.getSongByteLength(this.urlById(id), this.ytdlOptions)
            );
        } catch (e) {
            res.send({success: false});
            return;
        }

        const range = req.headers.range;
        if (range) {
            let [start, end] = range.substr(6).split('-');
            end = end ? end : fileSize - 1;

            Log.l('Youtube', 'Stream', id, 'byte range: ', start, end);

            let stream = ytdl(this.urlById(id), {
                quality: 'highestaudio',
                filter: 'audioonly',
                range: {start, end}
            });

            const chunkSize = (end - start) + 1;

            res.writeHead(206, {
                'Content-Range': `bytes ${start}-${end}/${fileSize}`,
                'Accept-Ranges': 'bytes',
                'Content-Length': chunkSize,
                'Content-Type': 'video/mp4',
            });

            stream.pipe(res);
        } else {
            let stream = ytdl(this.urlById(id), this.ytdlOptions);
            stream.on('error', error => {
                throw error;
            });
            stream.on("response", httpResponse => {
                let fileSize = parseInt(httpResponse.headers["content-length"]);

                res.writeHead(200, {
                    'Content-Length': fileSize,
                    'Content-Type': 'video/mp4',
                });

                stream.pipe(res);
            });
        }
    }

    decodeEntities(encodedString) {
        const translate_re = /&(nbsp|amp|quot|lt|gt);/g;
        const translate = {
            "nbsp": " ",
            "amp": "&",
            "quot": "\"",
            "lt": "<",
            "gt": ">"
        };
        return encodedString.replace(translate_re, function (match, entity) {
            return translate[entity];
        }).replace(/&#(\d+);/gi, function (match, numStr) {
            const num = parseInt(numStr, 10);
            return String.fromCharCode(num);
        });
    }

    async search(query, maxResults = 5, category) {
        let key = query + maxResults + category;
        if (!this.searchCache.hasOwnProperty(key))
            this.searchCache[key] = await this.searchYt(query, maxResults, category);

        return this.searchCache[key];
    }

    async searchYt(query, maxResults = 5, category) {
        const key = secrets.ytKey;
        Log.l('Youtube', "Search:", query);

        return new Promise((resolve, error) => {
            const opts = {
                maxResults,
                key: key,
                type: 'video'
            };
            if (category !== undefined)
                opts.videoCategoryId = category;

            youtubeSearch(query, opts, (err, results) => {
                if (err) error(err);
                if (results) resolve(results);
                else resolve("Not found, YoutubeSearch error");
            });
        });
    }
}

export default new Youtube();