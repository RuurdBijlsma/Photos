import {getSuggestions, search, text2media} from "./twimote.js";
import Database from "../../database/Database.js";
import seq from "sequelize";
import cred from "../../../res/auth/credentials.json";

const {Sequelize} = seq;
const {dbUser, dbPass, dbName} = cred;

async function test() {
    const db = new Sequelize(dbName, dbUser, dbPass, {
        host: 'localhost',
        dialect: 'postgres',
    });
    await Database.setDb(db);

    await text2media("R I OMEGALUL L U xD 😂");
    await text2media("PepePls hello peepoClap xD");

    let suggs = await getSuggestions("R I OMEG");
    console.log(suggs);
    let suggs2 = await getSuggestions("owow monka");
    console.log(suggs2);
    let res = await search("STE");
    console.log(res);
}

test().then(() => console.log('test complete'));