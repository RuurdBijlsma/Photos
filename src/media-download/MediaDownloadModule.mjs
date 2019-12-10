import ApiModule from "../ApiModule";
import Log from "../Log";
import path from 'path';
import fs from 'fs';
import Utils from "../Utils";
import crypto from 'crypto';

export default class SignalModule extends ApiModule {
    constructor() {
        super();
        this.baseDir = '/media/complete/';
        // this.baseDir = 'res/'; //For testing
        this.fakeDir = /media/;
        this.tokens = {};
    }

    setRoutes(app, io) {
        //this.fakeDir is route
        app.post(/\/media/, async (req, res) => {
            let auth = await Utils.checkAuthorization(req);
            if (!auth) {
                res.send(false);
            }
            let dir = this.validatePath(req.url, req);

            let itemType = await this.getItemType(dir);
            if (!itemType)
                res.send([]);
            if (itemType.directory) {
                fs.readdir(dir, async (err, items) => {
                    if (err)
                        return;

                    items = items.map(i => dir + '/' + i);

                    res.send(await Promise.all(items.map(item => this.parseItem(item))));
                });
            } else {
                res.send({error: "Get this file using the token and the /file GET endpoint"})
            }
        });

        app.get(/file/, async (req, res) => {
            if (!req.query.hasOwnProperty('token'))
                return;
            let token = req.query.token;
            if (this.tokens.hasOwnProperty(token)) {
                let filePath = path.resolve(this.tokens[token]);
                await res.download(filePath, path.basename(filePath));
            } else {
                res.send(false);
            }
        });
    }

    async parseItem(item) {
        let itemInfo = await this.getItemType(item);
        if (!itemInfo.directory) {
            let token = await this.getToken();
            this.tokens[token] = item;
            itemInfo.token = token;
            setTimeout(() => {
                delete this.tokens[token];
            }, 5 * 60 * 1000);//5 minutes
        }
        return itemInfo;
    }

    getToken() {
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

    validatePath(dir, req) {
        dir = decodeURIComponent(dir);
        if (!(dir.startsWith('/media/') &&
            (dir.includes('/tv') || dir.includes('/movies')))
        ) {
            Log.w("Download", 'Non media directory requested! req url:', req.url, 'directing to: ', this.baseDir);
            return this.baseDir;
        }
        dir = this.baseDir + dir.replace(/\/media\//gi, '');
        return dir;
    }

    async getItemType(path) {
        return new Promise((resolve, reject) => {
            fs.lstat(path, (err, result) => {
                if (err) {
                    Log.e("Download", err);
                    resolve(false);
                    return;
                }

                resolve({
                    directory: !result.isFile() && result.isDirectory(),
                    size: result.size,
                    path: path.replace(this.baseDir, this.fakeDir).replace(/\/\//gi, '/'),
                });
            });
        });
    }
}