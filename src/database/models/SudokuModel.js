import seq from "sequelize";
import {User} from "./UserModel.js";

const {DataTypes, Model} = seq;

export class Sudoku extends Model {
}

export function initSudoku(sequelize) {
    Sudoku.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        name: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        description: {
            type: DataTypes.STRING,
            allowNull: true,
        },
        puzzle: {
            type: DataTypes.JSON,
            allowNull: false,
        },
        difficulty: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        userDifficulty: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['name']},
            {unique: false, fields: ['difficulty']},
            {unique: false, fields: ['userDifficulty']},
        ],
    });
    Sudoku.belongsTo(User, {foreignKey: {allowNull: false}});

    return Sudoku;
}
