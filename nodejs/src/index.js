import ApiController from "./Controller.js";
import Commander from 'commander';
import pJson from '../package.json' assert {type: 'json'}

Commander
    .version(pJson.version)
    .option('-p, --port [value]', 'Server port')
    .parse(process.argv);

const port = Commander.port ? Commander.port : 3000;

await ApiController.start(port);


