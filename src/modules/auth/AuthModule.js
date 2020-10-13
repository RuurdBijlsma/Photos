import ApiModule from "../../ApiModule.js";
import Auth from "../../database/Auth.js";


export default class AuthModule extends ApiModule {
    constructor() {
        super();
    }

    setRoutes(app, io, db) {
        app.post('/auth', async (req, res) => {
            res.send(await Auth.checkRequest(req) !== false);
        });

        app.post('/auth/createUser', async (req, res) => {
            //Don't allow other users yet
            return res.sendStatus(401);
            try {
                let {user, password, email} = req.body;
                res.send(await Auth.createUser(user, email, password));
            } catch (e) {
                res.send(false);
            }
        });
    }
}