import dotenv from 'dotenv';

dotenv.config();
import ApiController from "./Controller.js";
import Commander from 'commander';
import pJson from '../package.json'

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .option('-d, --directory [value]', 'Directory to download to')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;

await ApiController.start(port);


