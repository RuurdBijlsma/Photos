import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaSuggestion extends Model {
}

export function initMediaSuggestion(sequelize) {
    MediaSuggestion.init({
        text: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        vector: {
            type: DataTypes.TSVECTOR,
            allowNull: false,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        count: {
            type: DataTypes.INTEGER,
            defaultValue: 0,
        },
    }, {
        timestamps: false,
        createdAt: false,
        updatedAt: false,
        sequelize,
        indexes: [
            {
                fields: ['text'],
                using: 'gin',
                operator: 'gin_trgm_ops',
            },
            {
                fields: ['vector'],
                using: 'gin',
                operator: 'tsvector_ops',
            },
            {unique: false, fields: ['type']},
            {unique: false, fields: ['count']},
        ],
    });

    return MediaSuggestion;
}
