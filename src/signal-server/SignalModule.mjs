// import ApiModule from "../ApiModule.mjs";
// import Log from "../Log.mjs";
//
// export default class SignalModule extends ApiModule {
//     constructor() {
//         super();
//         this.io = {};
//         this.rooms = [];
//     }
//
//     setRoutes(app, io) {
//         this.io = io;
//         app.get('/rooms', (req, res) => {
//             const nonHiddenApps = ['peercord'];
//
//             let rooms = this.rooms.filter(room => !room.hidden).map(room => ({
//                 id: room.id,
//                 appName: room.appName,
//                 userCount: room.sockets.length,
//             }));
//
//             for (let app of nonHiddenApps) {
//                 if (rooms.filter(room => room.appName === app).length === 0) {
//                     rooms.push({
//                         id: 'default',
//                         appName: app,
//                         userCount: 0,
//                     })
//                 }
//             }
//
//             res.send(rooms);
//             // return;
//             // let roomInfo = this.getAllRoomsInfo();
//             // if (!roomInfo.find(r => r.name === 'default'))
//             //     roomInfo.push({
//             //         name: 'default',
//             //         userCount: 0
//             //     });
//             // res.send(roomInfo.sort((a, b) => b.userCount - a.userCount));
//         });
//         io.on('connection', socket => {
//             socket.emit('socketId', socket.id);
//             socket.on('disconnect', () => {
//                 this.onDisconnect(socket);
//                 Log.l('Signal', `${socket.id} disconnected`);
//             });
//             socket.on('roomCount', room => {
//                 let roomCount = this.getRoomCount(room);
//                 Log.l('Signal', `${socket.id} requested roomCount ${roomCount}`);
//                 socket.emit('roomCount', roomCount);
//             });
//             socket.on('join', (roomId, appName, hidden = false) => {
//                 for (let room in this.getRooms(socket))
//                     socket.leave(room);
//                 Log.l('Signal', `${socket.id} joined room ${roomId}`);
//                 socket.join(roomId);
//                 let roomInfo = this.getRoomInfo(appName, roomId, hidden);
//                 roomInfo.sockets.push(socket);
//                 this.socketBroadcast(socket, 'initialize', socket.id);
//                 this.io.in(roomId).emit('roomCount', this.getRoomCount(roomId));
//             });
//             socket.on('leave', room => {
//                 Log.l('Signal', `${socket.id} left room ${room}`);
//                 this.onDisconnect(socket);
//                 socket.leave(room);
//             });
//             socket.on('broadcast', ([event, message]) => {
//                 this.socketBroadcast(socket, event, message);
//             });
//             socket.on('message', ([socketId, event, message]) => {
//                 Log.l('Signal', `${socket.id} send message to ${socketId}`);
//                 this.io.to(`${socketId}`).emit(event, [socket.id, message]);
//                 // this.io.to(`${socketId}`).emit('hey', 'I just met you');
//             });
//             Log.l('Signal', `${socket.id} connected`);
//         });
//     }
//
//     getRoomInfo(appName, roomId, hidden) {
//         let roomIndex = this.rooms.findIndex(room => room.appName === appName && room.id === roomId);
//         if (roomIndex === -1) {
//             this.rooms.push({id: roomId, appName, sockets: [], hidden});
//             roomIndex = this.rooms.length - 1;
//         }
//         return this.rooms[roomIndex];
//     }
//
//     getAllRoomsInfo() {
//         let rooms = Object.keys(this.io.sockets.adapter.rooms).filter(room => !(room in this.io.sockets.adapter.sids));
//         return rooms.map(room => {
//             return {
//                 name: room,
//                 userCount: this.getRoomCount(room)
//             }
//         });
//     }
//
//     getRoomCount(room) {
//         let roomClients = this.io.sockets.adapter.rooms[room];
//         if (roomClients && roomClients.length)
//             return roomClients.length;
//         return 0;
//     }
//
//     onDisconnect(socket) {
//         let room = this.rooms.find(room => room.sockets.includes(socket));
//         if (room) {
//             let newRoomCount = this.getRoomCount(room.id);
//             this.io.in(room.id).emit('roomCount', newRoomCount);
//             // Destroy refers to the peer that should now be destroyed
//             socket.to(room.id).emit('destroy', socket.id);
//             room.sockets.splice(room.sockets.indexOf(socket), 1);
//             if (newRoomCount === 0)
//                 this.rooms.splice(this.rooms.indexOf(room), 1);
//         }
//     }
//
//     getRooms(socket) {
//         let allRooms = Object.keys(this.io.sockets.adapter.sids[socket.id]);
//         Log.l(allRooms);
//         return allRooms.filter(r => r !== socket.id);
//     }
//
//     socketBroadcast(socket, event, message) {
//         Log.l(`${socket.id} broadcast: ${event}-${message}`);
//         let rooms = this.getRooms(socket);
//         for (let room of rooms)
//             socket.to(room).emit(event, message);
//     }
// }