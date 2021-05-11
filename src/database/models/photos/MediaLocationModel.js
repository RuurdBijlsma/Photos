import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaLocation extends Model {
}

export function initMediaLocation(sequelize) {
    MediaLocation.init({
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
        place: {
            type: DataTypes.TEXT,
            allowNull: true,
        },
        country: {
            type: DataTypes.TEXT,
            allowNull: true,
        },
        admin1: {
            type: DataTypes.TEXT,
            allowNull:true,
        },
        admin2: {
            type: DataTypes.TEXT,
            allowNull:true,
        },
        admin3: {
            type: DataTypes.TEXT,
            allowNull:true,
        },
        admin4: {
            type: DataTypes.TEXT,
            allowNull:true,
        },
    }, {
        timestamps:false,
        createdAt:false,
        updatedAt:false,
        sequelize,
        indexes: [
            {unique: false, fields: ['latitude']},
            {unique: false, fields: ['longitude']},
            {unique: false, fields: ['place']},
            {unique: false, fields: ['country']},
            {unique: false, fields: ['admin1']},
            {unique: false, fields: ['admin2']},
            {unique: false, fields: ['admin3']},
            {unique: false, fields: ['admin4']},
        ],
    });

    return MediaLocation;
}
