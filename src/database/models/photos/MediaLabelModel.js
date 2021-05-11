import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaLabel extends Model {
}

export function initMediaLabel(sequelize) {
    MediaLabel.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        text: {
            type: DataTypes.TEXT,
            allowNull: false,
        },
    }, {
        timestamps:false,
        createdAt:false,
        updatedAt:false,
        sequelize,
        indexes: [
            {unique: false, fields: ['text']},
        ],
    });

    return MediaLabel;
}
