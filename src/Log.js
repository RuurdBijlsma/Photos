export default class Log {
    constructor(tag) {
        this.tag = tag;
    }

    debug(...message) {
        console.debug(this.tag, ...message);
    }

    log(...message) {
        console.log(this.tag, ...message);
    }

    error(...message) {
        console.error(this.tag, ...message);
    }

    warn(...message) {
        console.warn(this.tag, ...message);
    }
}