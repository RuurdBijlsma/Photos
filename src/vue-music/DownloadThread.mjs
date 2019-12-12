import {Worker, isMainThread, parentPort, workerData} from 'worker_threads';

parentPort.postMessage("Hello :)");