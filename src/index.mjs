import ApiController from "./Controller";
import Commander from 'commander';
import pJson from '../package.json';

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-k, --key [value]', 'HTTPS key (ex: /etc/letsencrypt/live/domain.com/privkey.pem)')
    .option('-c, --cert [value]', 'HTTPS cert (ex: (ex: /etc/letsencrypt/live/domain.com/fullchain.pem)')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;
const key = Commander.key ? Commander.key : '/etc/letsencrypt/live/ruurd.dev/privkey.pem';
const cert = Commander.cert ? Commander.cert : '/etc/letsencrypt/live/ruurd.dev/fullchain.pem';

ApiController.start(port, key, cert);