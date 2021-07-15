export default class Log {
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
        }
    }
}
