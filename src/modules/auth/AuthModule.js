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
            try {
                let {user, password, email} = req.body;
                if (email !== 'ruurd@bijlsma.dev')
                    return res.sendStatus(401);
                res.send(await Auth.createUser(user, email, password));
            } catch (e) {
                res.send(false);
            }
        });

        app.post('/auth/changePassword', async (req, res) => {
            try {
                let {password, newPassword, email} = req.body;
                res.send(await Auth.changePassword(email, password, newPassword));
            } catch (e) {
                res.send(false);
            }
        });
    }
}