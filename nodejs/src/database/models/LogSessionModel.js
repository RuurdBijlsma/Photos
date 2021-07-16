import seq from "sequelize";

const {DataTypes, Model} = seq;

export class LogSession extends Model {
}

export function initLogSession(sequelize) {
    LogSession.init({
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
    }, {
        sequelize,
    });

    return LogSession;
}
