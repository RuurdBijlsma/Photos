export default class Room {
    constructor(appName, id, sockets = [], hidden = false, hashedPassword = '') {
        this.appName = appName;
        this.id = id;
        this.sockets = sockets;
        this.hidden = hidden;
        this.hashedPassword = hashedPassword;
    }

    get userCount(){
        return this.sockets.length;
    }

    get roomId() {
        return this.appName + '||' + this.id;
    }

    get secured() {
        return this.hashedPassword !== '';
    }

    static async create(appName, id, sockets = [], hidden = false, password = '') {
        return new Room(appName, id, sockets, hidden, password === '' ? '' : await bcrypt.hash(password, appName + id))
    }
}