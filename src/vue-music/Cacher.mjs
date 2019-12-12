import youtube from './Youtube.mjs';
import fs from 'fs';
import path from 'path';
import Log from "../Log.mjs";
import {Worker, isMainThread, parentPort, workerData} from 'worker_threads';

class Cacher {
    constructor() {
        this.songDirectory = 'res/vue-music/files/';
        this.cachingSongs = [];
        this.events = {};
    }

    fileExists(file) {
        return new Promise((resolve) => {
            fs.access(file, fs.F_OK, async (err) => {
                if (err)
                    resolve(false);
                resolve(true);
            });
        });
    }

    async cacheIfNotExists(query) {
        let fileExists = await this.fileExists(this.toPath(query));

        if (!fileExists)
            await this.cache(query);
    }

    toPath(query) {
        query = query.replace(/([^a-z0-9 ]+)/gi, '-');
        return path.join(this.songDirectory, query);
    }

    async cache(query) {
        return new Promise(async (resolve, reject) => {
            if (this.cachingSongs.includes(query)) {
                this.on('query' + query, () => resolve());
                return;
            }

            this.cachingSongs.push(query);

            let results = await youtube.search(query, 1);
            let id = results[0].id;
            let destinationPath = this.toPath(query);
            const worker = new Worker('./src/vue-music/DownloadThread.mjs', {
                workerData: {destinationPath, id},
            });
            worker.on('message', m => {
                if (m === 'Completed') {
                    //Download complete
                    resolve();
                    this.cachingSongs.splice(this.cachingSongs.indexOf(query), 1);
                    this.fire('query' + query);
                }
            });
            worker.on('exit', code => {
                if (code !== 0)
                    Log.e('Cacher', 'worker bad exit, code:', code);
            });

        });
    }

    fire(event) {
        if (this.events[event])
            for (let i = this.events[event].length - 1; i >= 0; i--)
                this.events[event][i]();
    }

    on(event, callback) {
        if (!this.events[event])
            this.events[event] = [];

        this.events[event].push(callback);
    }

    off(event, callback) {
        if (event in this.events)
            this.events[event].splice(this.events[event].indexOf(callback), 1);
        else
            Log.w('Cacher', `Trying to remove ${event} event, but it does not exist`);

    }
}

export default new Cacher();