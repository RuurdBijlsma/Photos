import fs from 'fs';
import * as path from "path";
import os from 'os';

const colors = {
    Reset: "\x1b[0m",
    Bright: "\x1b[1m",
    Dim: "\x1b[2m",
    Underscore: "\x1b[4m",
    Blink: "\x1b[5m",
    Reverse: "\x1b[7m",
    Hidden: "\x1b[8m",

    FgBlack: "\x1b[30m",
    FgRed: "\x1b[31m",
    FgGreen: "\x1b[32m",
    FgYellow: "\x1b[33m",
    FgBlue: "\x1b[34m",
    FgMagenta: "\x1b[35m",
    FgCyan: "\x1b[36m",
    FgWhite: "\x1b[37m",

    BgBlack: "\x1b[40m",
    BgRed: "\x1b[41m",
    BgGreen: "\x1b[42m",
    BgYellow: "\x1b[43m",
    BgBlue: "\x1b[44m",
    BgMagenta: "\x1b[45m",
    BgCyan: "\x1b[46m",
    BgWhite: "\x1b[47m"
};

export default class Log {
    static d(tag, ...message) {
        this.log(colors.FgCyan, 'DBG', tag, ...message);
    }

    static l(tag, ...message) {
        this.log(colors.FgWhite, 'LOG', tag, ...message);
    }

    static e(tag, ...message) {
        this.log(colors.FgRed, 'ERR', tag, ...message);
    }

    static w(tag, ...message) {
        this.log(colors.FgYellow, 'WRN', tag, ...message);
    }

    static log(color, type, tag, ...message) {
        console.warn(color + `[${tag}]`, ...message, colors.Reset);
    }
}