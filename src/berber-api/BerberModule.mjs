import sendMail from 'gmail-send';
import credentials from "../../res/berber-api/credentials.json";
import ApiModule from "../ApiModule";
import Log from "../Log";

export default class BerberModule extends ApiModule {
    setRoutes(app) {
        app.post('/mail/', async (req, res) => {
            let params = req.body;
            sendMail({
                from: params.from,
                replyTo: params.from,
                user: credentials.user,
                pass: credentials.pass,
                to: credentials.user,
                subject: `Berber's Bakery - ${params.subject}`,
                text: params.body,
            })({});
            Log.l("Sending mail from " + params.from);
            res.send({status: 'success'});
        });
}
}