import ApiModule from "../ApiModule.mjs";
import fetch from 'node-fetch';

export default class ReverseProxyModule extends ApiModule {
    setRoutes(app, _, params) {
        app.all('/proxy', async (req, res, next) => {
            if (!req.query.hasOwnProperty('url')) {
                res.send("Error: 'url' query param not set");
                return;
            }

            let proxyUrl = req.query['url'];
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