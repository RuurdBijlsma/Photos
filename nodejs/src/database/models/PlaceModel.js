import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Place extends Model {
}

export function initPlace(sequelize) {
    Place.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        text: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        isCode: {
            type: DataTypes.BOOLEAN,
            allowNull: false,
        },
    }, {
        timestamps: false,
        createdAt: false,
        updatedAt: false,
        sequelize,
        indexes: [
            {unique: false, fields: ['text']},
            {unique: false, fields: ['type']},
            {unique: false, fields: ['isCode']},
        ],
    });

    return Place;
}
