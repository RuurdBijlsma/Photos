import {Log} from "./database/models/LogModel.js";
import Database from "./database/DbInfo.js";
import DbInfo from "./database/DbInfo.js";

export default class Clog {
    constructor(tag) {
        this.tag = tag;
    }

    debug(...message) {
        this._log('debug', ...message);
    }

    log(...message) {
        this._log('log', ...message);
    }

    error(...message) {
        this._log('error', ...message);
    }

    warn(...message) {
        this._log('warn', ...message);
    }

    time(key) {
        this._log('time', key);
    }

    timeEnd(key) {
        this._log('timeEnd', key);
    }

    _log(type, ...args) {
        if (type.startsWith('time')) {
            console[type](...args);
        } else {
            console[type](`[${this.tag}]`, ...args);
            try {
                if (Database.isConnected)
                    Log.create({
                        type,
                        tag: this.tag,
                        stamp: Math.floor(performance.now() * 1000000),
                        message: args.join(' • '),
                        LogSessionId: DbInfo.session,
                    }).then();
            } catch (e) {
            }
        }
    }
}
