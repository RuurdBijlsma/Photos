import bcrypt from 'bcrypt';
import auth from '../res/authorization.json';
import Log from "./Log";

const saltRounds = 10;
export default class Utils {
    static checkAuthorization(req) {
        return new Promise((resolve, reject) => {

            if (!req.body.hasOwnProperty('password') ||
                !req.body.hasOwnProperty('user')) {
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

            let userHash = auth[user];
            bcrypt.compare(password, userHash, (err, result) => {
                if (err) {
                    reject(err);
                    failed();
                    return;
                }

                if (!result)
                    failed();

                resolve(result);
            });

        });
    }
}