import ApiController from "./Controller.js";
import Commander from 'commander';
import pJson from '../package.json';

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-d, --directory [value]', 'Directory to download to')
    .option('-k, --key [value]', 'HTTPS key (ex: /etc/letsencrypt/live/domain.com/privkey.pem)')
    .option('-c, --cert [value]', 'HTTPS cert (ex: (ex: /etc/letsencrypt/live/domain.com/fullchain.pem)')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;

ApiController.start(port, Commander.key, Commander.cert);


