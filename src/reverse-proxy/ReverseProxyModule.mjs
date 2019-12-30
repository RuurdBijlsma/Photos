import ApiModule from "../ApiModule.mjs";
import fetch from 'node-fetch';

export default class ReverseProxyModule extends ApiModule {
    setRoutes(app, _, params) {
        app.all('/proxy', async (req, res, next) => {
            if (!req.headers.hasOwnProperty('x-proxy-url')) {
                res.send("Error: 'x-proxy-url header not set");
                return;
            }

            let proxyUrl = req.headers['x-proxy-url'];
            delete req.headers['x-proxy-url'];
            let options = {
                method: req.method,
                headers: req.headers,
                body: req.body,
                rejectUnauthorized: false,
            };

            if (options.method === 'GET' || options.method === 'HEAD')
                delete options.body;

            try {
                let response = await fetch(proxyUrl, options);
                res.send(response);
            } catch (e) {
                res.send({error: e.message});
            }
        });
    }
}