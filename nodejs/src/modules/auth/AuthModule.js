import ApiModule from "../../ApiModule.js";
import Auth from "../../database/Auth.js";


export default class AuthModule extends ApiModule {
    constructor() {
        super();
    }

    setRoutes(app, db) {
        app.post('/auth', async (req, res) => {
            let authResult = await Auth.checkRequest(req);
            res.send(authResult);
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
                let {auth, newPassword} = req.body;
                res.send(await Auth.changePassword(auth.email, auth.password, newPassword));
            } catch (e) {
                res.send(false);
            }
        });
    }
}
