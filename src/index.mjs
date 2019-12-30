import ApiController from "./Controller.mjs";
import Commander from 'commander';
import pJson from '../package.json';
import Log from "./Log.mjs";

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-d, --directory [value]', 'Directory to download to')
    .option('-k, --key [value]', 'HTTPS key (ex: /etc/letsencrypt/live/domain.com/privkey.pem)')
    .option('-c, --cert [value]', 'HTTPS cert (ex: (ex: /etc/letsencrypt/live/domain.com/fullchain.pem)')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;
const key = Commander.key ? Commander.key : '/etc/letsencrypt/live/ruurd.dev/privkey.pem';
const cert = Commander.cert ? Commander.cert : '/etc/letsencrypt/live/ruurd.dev/fullchain.pem';

const directory = Commander.directory ? Commander.directory : '/home/ruurd/music';
const params = {directory};

ApiController.start(port, key, cert, params);


