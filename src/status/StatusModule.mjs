import sendMail from 'gmail-send';
import credentials from "../../res/berber-api/credentials.json";
import ApiModule from "../ApiModule";
import {exec} from "child_process";

export default class StatusModule extends ApiModule {
    setRoutes(app) {
        app.post('/status/', async (req, res) => {
            exec('sensors', (err, stdout, stderr) => {
                if (err) {
                    // node couldn't execute the command
                    console.log(err);
                    return;
                }

                let result = {};
                let words = stdout
                    .split('\n')
                    .filter(c => c.includes('°C'))
                    .map(l => l.replace(/\((.*?)\)/gi, '')
                        .split('  ')
                        .filter(w => w.length > 0)
                        .map(w => w
                            .trim()
                            .replace(/°C/gi, '')
                            .replace(/:/gi, '')
                        )
                    );
                for (let [key, value] of words)
                    result[key] = +value;

                console.log('stderr', stderr);
                res.send(result);
            });
        });
    }
}