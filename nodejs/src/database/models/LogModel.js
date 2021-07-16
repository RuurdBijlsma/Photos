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
        stamp: {
            type: DataTypes.BIGINT,
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
            {unique: false, fields: ['stamp']},
            {unique: false, fields: ['message']},
        ],
    });

    return Log;
}
