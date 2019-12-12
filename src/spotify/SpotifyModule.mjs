import pgp from 'pg-promise';
import ApiModule from "../ApiModule.mjs";
import fetch from 'node-fetch';
import secret from '../../res/spotify/secret.json';

export default class SpotifyModule extends ApiModule {
    constructor() {
        super();
        this.authString = Buffer.from(secret.client_id + ':' + secret.client_secret).toString('base64');
    }

    setRoutes(app, io) {
        app.post('/refresh', async (req, res) => {
            let refreshToken = req.body.refresh_token;

            let response = await fetch('https://accounts.spotify.com/api/token', {
                method: 'post',
                body: `grant_type=refresh_token&refresh_token=${refreshToken}&client_id=${secret.client_id}&client_secret=${secret.client_secret}`,
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                }
            });
            res.send(await response.text());
        });

        app.post('/token', async (req, res) => {
            let redirectUrl = req.body.redirect_url;
            let authCode = req.body.auth_code;

            let response = await fetch('https://accounts.spotify.com/api/token', {
                method: 'post',
                body: `grant_type=authorization_code&code=${authCode}&redirect_uri=${redirectUrl}&client_id=${secret.client_id}&client_secret=${secret.client_secret}`,
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                }
            });
            res.send(await response.text());
        });
    }
}