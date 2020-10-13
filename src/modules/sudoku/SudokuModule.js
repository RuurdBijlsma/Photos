import ApiModule from "../../ApiModule.js";
import {Sudoku} from "../../database/models/SudokuModel.js";
import Auth from "../../database/Auth.js";

export default class SudokuModule extends ApiModule {
    constructor() {
        super();
    }

    setRoutes(app, io, db) {
        app.get('/sudoku/:id', async (req, res) => {
            try {
                let id = req.params.id;
                if (id === undefined)
                    return res.sendStatus(400);

                let sudoku = await Sudoku.findOne({where: {id}});
                if (sudoku === null)
                    return res.sendStatus(404);

                let user = await sudoku.getUser();
                return res.send({sudoku, user: {name: user.name, id: user.id}});
            } catch (e) {
                res.sendStatus(400);
            }
        });
        app.patch('/sudoku/:id', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user)
                return res.sendStatus(401);

            let id = req.params.id;
            if (id) {
                let sudoku = await Sudoku.findOne({where: {id}});
                if (sudoku === null)
                    return res.sendStatus(404);

                if (sudoku.UserId !== user.id)
                    return res.sendStatus(401);

                for (let key in req.body)
                    if (sudoku.dataValues.hasOwnProperty(key))
                        sudoku[key] = req.body[key];

                await sudoku.save();
                return res.sendStatus(200)
            } else {
                res.sendStatus(401);
            }
        });
        app.delete('/sudoku/:id', async (req, res) => {
            let user = await Auth.checkRequest(req);
            if (!user)
                return res.sendStatus(401);

            let id = req.params.id;
            if (id) {
                let sudoku = await Sudoku.findOne({where: {id}});
                if (sudoku === null)
                    return res.sendStatus(404);

                if (sudoku.UserId !== user.id)
                    return res.sendStatus(401);
                await sudoku.destroy();
                return res.sendStatus(200)
            } else {
                res.sendStatus(401);
            }
        });
        app.post('/sudoku/', async (req, res) => {
            console.log("POSTING SUDOKU");
            let user = await Auth.checkRequest(req);
            if (!user)
                return res.sendStatus(401);
            try {
                let {name, description, userDifficulty, background, puzzle} = req.body;
                let sudoku = await Sudoku.create({
                    name,
                    description,
                    difficulty: -1,
                    userDifficulty,
                    puzzle,
                    UserId: user.id,
                });

                res.status(201);
                return res.send(sudoku.id.toString());
            } catch (e) {
                res.sendStatus(400);
            }
        });
    }
}