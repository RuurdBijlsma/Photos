import sendMail from 'gmail-send';
import credentials from "../../res/berber-api/credentials.json";
import ApiModule from "../ApiModule.mjs";
import {exec} from "child_process";
import Log from "../Log.mjs";
import si from 'systeminformation';
import Utils from "../Utils.mjs";

export default class StatusModule extends ApiModule {
    setRoutes(app) {
        app.post('/status/', async (req, res) => {
            let auth = await Utils.checkAuthorization(req);
            if (!auth) {
                res.send(false);
                return;
            }

            // noinspection ES6MissingAwait
            let promises = [si.cpuTemperature(), si.currentLoad(), si.mem(), si.fsSize(), si.networkStats()];
            let [temperature, load, memory, storage, network] = await Promise.all(promises);
            res.send({temperature, load, memory, storage, network});
        });
    }
}