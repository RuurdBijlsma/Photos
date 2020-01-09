import ApiModule from "../ApiModule.mjs";
import fetch from 'node-fetch';
import whiteList from '../../res/reverse-proxy/whitelist.json';

export default class ReverseProxyModule extends ApiModule {
    setRoutes(app, _, params) {
        app.all('/proxy', async (req, res, next) => {


            if (!req.query.hasOwnProperty('url')) {
                res.send("Error: 'url' query param not set");
                return;
            }

            let proxyUrl = req.query['url'];
            if (!whiteList.some(url => proxyUrl.startsWith(url))) {
                res.send("Error: url not in whitelist");
                return;
            }
            let options = {
                method: req.method,
                headers: req.headers,
                body: req.body,
            };

            for (let header in options.headers)
                if (options.headers.hasOwnProperty(header) &&
                    (header.toLowerCase().includes('host') || header.toLowerCase().includes('content-length')))
                    delete options.headers[header];

            if (options.method === 'GET' || options.method === 'HEAD')
                delete options.body;

            try {
                let response = await fetch(proxyUrl, options);
                response.headers.forEach((val, key) => {
                    if (!key.toLowerCase().includes('encoding'))
                        res.append(key, val);
                });
                res.send(await response.buffer());
            } catch (e) {
                res.send({error: e.message});
            }
        });
    }
}