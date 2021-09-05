import seq from "sequelize";

const {DataTypes, Model} = seq;

export class Album extends Model {
}

export function initAlbum(sequelize) {
    Album.init({
        id: {
            type: DataTypes.STRING,
            primaryKey: true,
        },
        name: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        cover: {
            type: DataTypes.STRING,
            allowNull: false,
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['name']},
        ],
    });

    return Album;
}
