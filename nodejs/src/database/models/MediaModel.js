import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Media extends Model {
}

export function initMedia(sequelize) {
    Media.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.STRING,
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
        filename: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true,
        },
        filePath: {
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
            type: DataTypes.BIGINT,
            allowNull: true,
        },
        bytes: {
            type: DataTypes.BIGINT,
            allowNull: false,
        },
        createDate: {
            type: DataTypes.DATE,
            allowNull: true,
        },
        createDateString: {
            type: DataTypes.STRING,
            allowNull: true,
        },
        exif: {
            type: DataTypes.JSONB,
            allowNull: false,
        },
        vectorA: {
            type: DataTypes.TSVECTOR,
            allowNull: false,
        },
        vectorB: {
            type: DataTypes.TSVECTOR,
            allowNull: false,
        },
        vectorC: {
            type: DataTypes.TSVECTOR,
            allowNull: false,
        },
        vector: {
            type: DataTypes.TSVECTOR,
            allowNull: true,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['type']},
            {unique: false, fields: ['subType']},
            {unique: false, fields: ['createDate']},
            {unique: false, fields: ['bytes']},
            {unique: false, fields: ['durationMs']},
            {unique: false, fields: ['width']},
            {unique: false, fields: ['height']},
            {unique: true, fields: ['filename']},
            {unique: true, fields: ['filePath']},
            {
                fields: ['vectorA'],
                using: 'gin',
                operator: 'tsvector_ops',
            },
            {
                fields: ['vectorB'],
                using: 'gin',
                operator: 'tsvector_ops',
            },
            {
                fields: ['vectorC'],
                using: 'gin',
                operator: 'tsvector_ops',
            },
            {
                fields: ['vector'],
                using: 'gin',
                operator: 'tsvector_ops',
            },
        ],
    });

    return Media;
}
