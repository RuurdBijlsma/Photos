import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Suggestion extends Model {
}

export function initSuggestion(sequelize) {
    Suggestion.init({
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
        data:{
            type: DataTypes.STRING,
            allowNull: true,
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
            {unique: false, fields: ['data']},
            {unique: false, fields: ['type']},
            {unique: false, fields: ['count']},
        ],
    });

    return Suggestion;
}
