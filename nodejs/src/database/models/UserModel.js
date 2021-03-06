import seq from "sequelize";

const {DataTypes, Model} = seq;
export class User extends Model {
}
export function initUser(sequelize) {
    User.init({
        // Model attributes are defined here
        id: {
            type: DataTypes.INTEGER,
            autoIncrement: true,
            primaryKey: true,
        },
        name: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        email: {
            type: DataTypes.STRING,
            allowNull: false,
            unique: true,
        },
        password: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        mapboxToken:{
            type: DataTypes.STRING,
            allowNull: false,
            defaultValue: '',
        },
    }, {
        sequelize,
        indexes: [
            {unique: false, fields: ['name']},
            {unique: false, fields: ['email']},
            {unique:false, fields: ['mapboxToken']}
        ],
    });

    return User;
}
