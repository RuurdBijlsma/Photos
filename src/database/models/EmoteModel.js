import seq from "sequelize";

const {DataTypes, Model} = seq;
export class Emote extends Model {
}
export function initEmote(sequelize) {
    Emote.init({
        // Model attributes are defined here
        name: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        ratio: {
            type: DataTypes.FLOAT,
            allowNull: false,
        },
        animated: {
            type: DataTypes.BOOLEAN,
            allowNull: false,
        },
        duration: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        frames: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        url: {
            type: DataTypes.STRING,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['name']},
        ],
    });

    return Emote;
}
