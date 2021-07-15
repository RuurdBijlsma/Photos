import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Glossary extends Model {
}

export function initGlossary(sequelize) {
    Glossary.init({
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
        level: {
            type: DataTypes.INTEGER,
            allowNull: false,
        },
    }, {
        timestamps: false,
        createdAt: false,
        updatedAt: false,
        sequelize,
        indexes: [
            {unique: false, fields: ['text']},
            {unique: false, fields: ['level']},
        ],
    });

    return Glossary;
}
