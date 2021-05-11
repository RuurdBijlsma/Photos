import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaClassification extends Model {
}

export function initMediaClassification(sequelize) {
    MediaClassification.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        confidence: {
            type: DataTypes.FLOAT,
            allowNull: false,
        },
    }, {
        timestamps: false,
        createdAt: false,
        updatedAt: false,
        sequelize,
        indexes: [
            {unique: false, fields: ['confidence']},
        ],
    });

    return MediaClassification;
}
