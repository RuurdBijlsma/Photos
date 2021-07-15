import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Log extends Model {
}

export function initLog(sequelize) {
    Log.init({
        filePath: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        id: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        reason: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        error: {
            type: DataTypes.JSONB,
            allowNull: true,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['type']},
            {unique: false, fields: ['reason']},
        ],
    });

    return Log;
}
