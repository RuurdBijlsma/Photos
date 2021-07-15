import {getToken} from "../utils.js";

class DbInfo{
    constructor() {
        this.isConnected = false;
        this.session = '';
        getToken(10).then(t => this.session = t);
    }
}

export default new DbInfo();
