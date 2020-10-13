import {User} from "./models/UserModel.js";
import bcrypt from "bcrypt";
import Log from "../Log.js";

class Auth {
    constructor() {
    }

    async checkRequest(req) {
        try {
            let {email, password} = req.body.auth;
            return await this.check(email, password);
        } catch (e) {
            return false;
        }
    }

    async check(email, password) {
        try {
            let dbUser = await User.findOne({where: {email}});
            if (dbUser !== null)
                if (await bcrypt.compare(password, dbUser.password)) {
                    return dbUser;
                } else {
                    return false;
                }
        } catch (e) {
            Log.e('Auth', 'Error in /auth', e.message);
        }
        return false;
    }

    async changePassword(email, password, newPassword) {
        let user = await this.check(email, password);
        if (user !== false) {
            user.password = newPassword;
            await user.save();
        }
        return false;
    }

    async createUser(user, email, password) {
        try {
            let salt = await bcrypt.genSalt(10);
            let hashed = await bcrypt.hash(password, salt);
            await User.create({name: user, email, password: hashed});
            return true;
        } catch (e) {
            Log.e('Auth', 'Error in /createUser', e.message);
            return false;
        }
    }
}

export default new Auth();