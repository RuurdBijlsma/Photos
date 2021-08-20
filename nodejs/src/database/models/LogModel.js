import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Log extends Model {
}

export function initLog(sequelize) {
    Log.init({
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        tag: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        order: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        message: {
            type: DataTypes.TEXT,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['type']},
            {unique: false, fields: ['tag']},
            {unique: false, fields: ['order']},
            {unique: false, fields: ['message']},
        ],
    });

    return Log;
}
