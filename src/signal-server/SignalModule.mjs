import ApiModule from "../ApiModule.mjs";
import Log from "../Log.mjs";

export default class SignalModule extends ApiModule {
    constructor() {
        super();
        this.io = {};
        this.socketRooms = {};
    }

    setRoutes(app, io) {
        this.io = io;
        app.get('/rooms', (req, res) => {
            let roomInfo = this.getAllRoomsInfo();
            if (!roomInfo.find(r => r.name === 'default'))
                roomInfo.push({
                    name: 'default',
                    userCount: 0
                });
            res.send(roomInfo.sort((a, b) => b.userCount - a.userCount));
        });
        io.on('connection', socket => {
            socket.emit('socketId', socket.id);
            socket.on('disconnect', () => {
                this.onDisconnect(socket);
                Log.l('Signal', `${socket.id} disconnected`);
            });
            socket.on('roomCount', room => {
                let roomCount = this.getRoomCount(room);
                Log.l('Signal', `${socket.id} requested roomCount ${roomCount}`);
                socket.emit('roomCount', roomCount);
            });
            socket.on('join', room => {
                for (let room in this.getRooms(socket))
                    socket.leave(room);
                Log.l('Signal', `${socket.id} joined room ${room}`);
                socket.join(room);
                this.socketRooms[socket.id] = room;
                this.socketBroadcast(socket, 'initialize', socket.id);
                this.io.in(room).emit('roomCount', this.getRoomCount(room));
            });
            socket.on('leave', room => {
                Log.l('Signal', `${socket.id} left room ${room}`);
                this.onDisconnect(socket);
                socket.leave(room);
            });
            socket.on('broadcast', ([event, message]) => {
                this.socketBroadcast(socket, event, message);
            });
            socket.on('message', ([socketId, event, message]) => {
                Log.l('Signal', `${socket.id} send message to ${socketId}:this.io.to(socketId).emit('signal', 'test2');`);
                this.io.to(`${socketId}`).emit(event, [socket.id, message]);
                // this.io.to(`${socketId}`).emit('hey', 'I just met you');
            });
            Log.l('Signal', `${socket.id} connected`);
        });
    }

    getAllRoomsInfo() {
        let rooms = Object.keys(this.io.sockets.adapter.rooms).filter(room => !(room in this.io.sockets.adapter.sids));
        return rooms.map(room => {
            return {
                name: room,
                userCount: this.getRoomCount(room)
            }
        });
    }

    getRoomCount(room) {
        let roomClients = this.io.sockets.adapter.rooms[room];
        if (roomClients && roomClients.length)
            return roomClients.length;
        return 0;
    }

    onDisconnect(socket) {
        let room = this.socketRooms[socket.id];
        if (room) {
            this.io.in(room).emit('roomCount', this.getRoomCount(room));
            socket.to(room).emit('destroy', socket.id);
        }
        delete this.socketRooms[socket.id];
    }

    getRooms(socket) {
        let allRooms = Object.keys(this.io.sockets.adapter.sids[socket.id]);
        Log.l(allRooms);
        return allRooms.filter(r => r !== socket.id);
    }

    socketBroadcast(socket, event, message) {
        Log.l(`${socket.id} broadcast: ${event}-${message}`);
        let rooms = this.getRooms(socket);
        for (let room of rooms)
            socket.to(room).emit(event, message);
    }
}