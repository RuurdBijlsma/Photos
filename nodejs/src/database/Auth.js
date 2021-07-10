import {User} from "./models/UserModel.js";
import bcrypt from "bcrypt";
import Log from "../Log.js";

const console = new Log("Auth");

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
            console.error('Error in /auth', e.message);
        }
        return false;
    }

    async changePassword(email, password, newPassword) {
        let user = await this.check(email, password);
        if (user !== false) {
            let salt = await bcrypt.genSalt(10);
            user.password = await bcrypt.hash(newPassword, salt);
            await user.save();
            return true;
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
            console.error('Error in /createUser', e.message);
            return false;
        }
    }
}

export default new Auth();
