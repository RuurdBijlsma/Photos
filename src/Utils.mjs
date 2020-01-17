import bcrypt from 'bcrypt';
import auth from '../res/authorization.json';
import Log from "./Log.mjs";

export default class Utils {
    static bytesToReadable(bytes) {
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

    static checkAuthorization(req) {
        return new Promise(async (resolve, reject) => {

            if (!req.body.hasOwnProperty('password') ||
                !req.body.hasOwnProperty('user')) {
                console.log("Auth body incomplete", req.body);
                resolve(false);
                return;
            }


            let {password, user} = req.body;
            let failed = () => Log.l("Utils", "Failed login attempt from " + user);

            if (!auth.hasOwnProperty(user)) {
                resolve(false);
                failed();
                return;
            }

            let userHashesPassword = auth[user];

            console.log("Attempted login to user", user, "using password:", password, "user hashed pw is", userHashesPassword);

            let result = await bcrypt.compare(password, userHashesPassword);
            console.log("login result", result);
            if (!result)
                failed();

            resolve(result);

        });
    }
}