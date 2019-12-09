import ApiModule from "../ApiModule";
import Log from "../Log";
import path from 'path';
import fs from 'fs';
import Utils from "../Utils";

export default class SignalModule extends ApiModule {
    constructor() {
        super();
        this.baseDir = '/media/complete/';
        this.fakeDir = /media/;
    }

    setRoutes(app, io) {
        //this.fakeDir is route
        app.post(/\/media/, async (req, res) => {
            let auth = await Utils.checkAuthorization(req);
            if(!auth){
                res.send(false);
            }
            let dir = this.validatePath(req.url, req);

            let itemType = await this.getItemType(dir);
            if(!itemType)
                res.send([]);
            if (itemType.directory) {
                fs.readdir(dir, async (err, items) => {
                    if (err)
                        return;

                    items = items.map(i => dir + '/' + i);

                    res.send(await Promise.all(items.map(item => this.getItemType(item))));
                });
            } else {
                res.sendFile(path.resolve(dir));
            }
        });
    }

    validatePath(dir, req) {
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