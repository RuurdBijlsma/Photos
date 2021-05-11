import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaItem extends Model {
}

export function initMediaItem(sequelize) {
    MediaItem.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        subType: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        fileName: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true,
        },
        width: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        height: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        durationMs: {
            type: DataTypes.INTEGER,
            allowNull: true,
        },
        bytes: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
        createDate: {
            type: DataTypes.DATE,
            allowNull: true,
        },
        exif: {
            type: DataTypes.JSONB,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['type']},
            {unique: false, fields: ['subType']},
        ],
    });

    return MediaItem;
}
