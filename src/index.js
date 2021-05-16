import ApiController from "./Controller.js";
import Commander from 'commander';
import pJson from '../package.json'
import geocode from "./modules/photos/reverse-geocode.js";

console.log("Initializing geocoder");
await geocode({latitude: 50, longitude: 5});
console.log("Initialized geocoder");

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-d, --directory [value]', 'Directory to download to')
    .option('-k, --key [value]', 'HTTPS key (ex: /etc/letsencrypt/live/domain.com/privkey.pem)')
    .option('-c, --cert [value]', 'HTTPS cert (ex: (ex: /etc/letsencrypt/live/domain.com/fullchain.pem)')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;

await ApiController.start(port, Commander.key, Commander.cert);


