import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Location extends Model {
}

export function initLocation(sequelize) {
    Location.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        latitude: {
            type: DataTypes.FLOAT,
            allowNull: false,
        },
        longitude: {
            type: DataTypes.FLOAT,
            allowNull: false,
        },
        altitude: {
            type: DataTypes.FLOAT,
            allowNull: true,
        },
    }, {
        timestamps:false,
        createdAt:false,
        updatedAt:false,
        sequelize,
        indexes: [
            {unique: false, fields: ['latitude']},
            {unique: false, fields: ['longitude']},
            {unique: false, fields: ['altitude']},
        ],
    });

    return Location;
}
