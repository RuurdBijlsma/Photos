import ApiModule from "../../ApiModule.js";
import si from 'systeminformation';
import Auth from "../../database/Auth.js";

export default class StatusModule extends ApiModule {
    setRoutes(app) {
        app.post('/status/', async (req, res) => {
            let auth = await Auth.checkRequest(req);
            if (!auth)
                return res.sendStatus(401);

            // noinspection ES6MissingAwait
            let promises = [si.cpuTemperature(), si.currentLoad(), si.mem(), si.fsSize(), si.networkStats()];
            let [temperature, load, memory, storage, network] = await Promise.all(promises);
            res.send({temperature, load, memory, storage, network});
        });
    }
}