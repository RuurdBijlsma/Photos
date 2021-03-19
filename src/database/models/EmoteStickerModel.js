import seq from "sequelize";

const {DataTypes, Model} = seq;
export class EmoteSticker extends Model {
}
export function initEmoteSticker(sequelize) {
    EmoteSticker.init({
        // Model attributes are defined here
        text: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        sticker: {
            type: DataTypes.STRING,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['text']},
        ],
    });

    return EmoteSticker;
}
