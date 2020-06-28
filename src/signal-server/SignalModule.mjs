import ApiModule from "../ApiModule.mjs";
import Log from "../Log.mjs";
import Room from "./Room.mjs";
import bcrypt from 'bcrypt'

export default class SignalModule extends ApiModule {
    constructor() {
        super();
        this.io = {};
        this.rooms = [];
    }

    setRoutes(app, io) {
        this.io = io;
        app.get('/rooms', (req, res) => {
            let rooms = this.getSurfaceLevelRooms()

            res.send(rooms);
        });
        io.on('connection', socket => {
            socket.emit('socket-id', socket.id);

            socket.on('watch-rooms', appName => {
                socket.join(appName + 'watcher');
                socket.emit('watch-update', this.getSurfaceLevelRooms(appName));
            });
            socket.on('disconnect', () => {
                let room = this.getRoomBySocket(socket);
                this.onDisconnect(socket);
                if (room)
                    this.updateRoom(room.appName);
                Log.l('Signal', `${socket.id} disconnected`);
            });
            socket.on('room-count', (appName, roomId) => {
                let room = this.getRoom(appName, roomId);
                Log.l('Signal', `${socket.id} requested roomCount ${room.userCount}`);
                socket.emit('room-count', room.userCount);
            });
            socket.on('create', async (appName, roomId, password = '', hidden = false) => {
                await this.createRoom(socket, appName, roomId, password, hidden);
            });
            socket.on('join', async (appName, roomId, password = '') => {
                let room = this.getRoom(appName, roomId);
                if (room === null) {
                    await this.createRoom(socket, appName, roomId, password, false);
                    return;
                }
                if (await bcrypt.compare(password, room.hashedPassword)) {
                    socket.emit('room-wrong-password');
                    return;
                }
                this.leaveCurrentRoom(socket);
                Log.l('Signal', `${socket.id} joined room ${roomId}`);
                socket.join(room.roomId);
                room.sockets.push(socket);
                Log.l('Signal', `${socket.id} has been pushed to room (in join): ${roomId}`);
                this.updateRoom(appName);
                this.socketBroadcast(socket, 'initialize', 'client', socket.id);
                this.io.in(room.roomId).emit('room-count', room.userCount);
            });
            socket.on('leave', (appName, roomId) => {
                let room = this.getRoom(appName, roomId);
                Log.l('Signal', `${socket.id} left room ${roomId}`);
                this.onDisconnect(socket);
                socket.leave(room.roomId);
                this.updateRoom(appName);
            });
            // Todo improve security in message and broadcast event
            socket.on('broadcast', (event, ...message) => {
                this.socketBroadcast(socket, event, ...message);
            });
            socket.on('message', (socketId, event, ...message) => {
                Log.l('Signal', `${socket.id} send message to ${socketId}`);
                this.io.to(`${socketId}`).emit(event, socket.id, ...message);
            });
            Log.l('Signal', `${socket.id} connected`);
        });
    }

    async createRoom(socket, appName, roomId, password, hidden) {
        if (this.getRoom(appName, roomId) !== null) {
            socket.emit('room-already-exists');
            return;
        }
        this.leaveCurrentRoom(socket);
        Log.l('Signal', `${socket.id} created room ${roomId}`);
        let room = await Room.create(appName, roomId, [socket], hidden, password);
        socket.join(room.roomId);
        this.rooms.push(room);
        this.updateRoom(appName);
        Log.l('Signal', `${socket.id} has been pushed to room (in create): ${roomId}`);
        this.io.in(room.roomId).emit('room-count', room.userCount);
    }

    updateRoom(appName) {
        this.io.to(appName + 'watcher').emit('watch-update', this.getSurfaceLevelRooms(appName))
    }

    getRoom(appName, roomId) {
        let roomIndex = this.rooms.findIndex(room => room.appName === appName && room.id === roomId);
        if (roomIndex === -1) {
            return null;
        }
        return this.rooms[roomIndex];
    }

    getRoomBySocket(socket) {
        return this.rooms.find(room => room.sockets.findIndex(s => s.id === socket.id) !== -1);
    }

    getSurfaceLevelRooms(appName = false) {
        let rooms = this.rooms.filter(room => !room.hidden).map(room => ({
            id: room.id,
            appName: room.appName,
            userCount: room.sockets.length,
            secured: room.secured,
        }))

        const nonHiddenApps = ['peercord'];
        for (let app of nonHiddenApps) {
            if (rooms.filter(room => room.appName === app).length === 0) {
                rooms.push({
                    id: 'default',
                    appName: app,
                    userCount: 0,
                    secured: false,
                })
            }
        }

        if (appName !== false)
            return rooms.filter(room => room.appName === appName);
        return rooms;
    }

    onDisconnect(socket) {
        let room = this.getRoomBySocket(socket);
        if (room) {
            // Destroy refers to the peer that should now be destroyed
            socket.to(room.roomId).emit('destroy', socket.id);
            room.sockets.splice(room.sockets.findIndex(s => s.id === socket.id), 1);
            if (room.userCount === 0)
                this.rooms.splice(this.rooms.indexOf(room), 1);
            else
                this.io.in(room.id).emit('room-count', room.userCount);
        }
    }

    leaveCurrentRoom(socket) {
        Log.l('Signal', `${socket.id} disconnected and left current room`);
        let room = this.getRoomBySocket(socket);
        if (room) {
            socket.leave(room.roomId);
            this.onDisconnect(socket);
        }
    }

    socketBroadcast(socket, event, ...message) {
        Log.l('Signal', `${socket.id} broadcast: ${event} ${message}`);
        let room = this.getRoomBySocket(socket);
        socket.to(room.roomId).emit(event, ...message);
    }
}