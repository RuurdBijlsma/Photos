import ApiController from "./Controller.mjs";
import Commander from 'commander';
import pJson from '../package.json';
import bcrypt from 'bcrypt';

//If password needs to be changed
// bcrypt.hash('Xbv5qCdMpb5jNSz', 10, (err, hash) => {
//     if (err)
//         console.err("No hash :(");
//     else
//         console.log("HASHED PW: " + hash);
// });

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


