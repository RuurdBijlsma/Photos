import Database from "../../../database/Database.js";
import {User} from "../../../database/models/UserModel";
import bcrypt from "bcrypt";

await Database.initDb();
let email = 'ruurd@bijlsma.dev';
let newPassword = '';
let user = await User.findOne({where: {email}})
if (user) {
    let salt = await bcrypt.genSalt(10);
    user.password = await bcrypt.hash(newPassword, salt);
    await user.save();
    console.log("changed password");
}
console.log("didn't change password");
