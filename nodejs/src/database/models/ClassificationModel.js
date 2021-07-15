import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Classification extends Model {
}

export function initClassification(sequelize) {
    Classification.init({
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

    return Classification;
}
