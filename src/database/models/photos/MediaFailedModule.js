import seq from "sequelize";

const {DataTypes, Model} = seq;

export class MediaFailed extends Model {
}

export function initMediaFailed(sequelize) {
    MediaFailed.init({
        filePath: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        type: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        reason: {
            type: DataTypes.JSONB,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['type']},
            {unique: false, fields: ['reason']},
        ],
    });

    return MediaFailed;
}
