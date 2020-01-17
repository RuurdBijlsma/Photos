import ApiController from "./Controller.mjs";
import Commander from 'commander';
import pJson from '../package.json';
import bcrypt from 'bcrypt';

//If password needs to be changed
(async () => {
    // let hash = await bcrypt.hash('pupernoot', 10);
    // console.log(hash);
})();

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-d, --directory [value]', 'Directory to download to')
    .option('-k, --key [value]', 'HTTPS key (ex: /etc/letsencrypt/live/domain.com/privkey.pem)')
    .option('-c, --cert [value]', 'HTTPS cert (ex: (ex: /etc/letsencrypt/live/domain.com/fullchain.pem)')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;

const directory = Commander.directory ? Commander.directory : '/home/ruurd/music';
const params = {directory};

ApiController.start(port, Commander.key, Commander.cert, params);


