import {initMediaClassification, MediaClassification} from "./MediaClassificationModel.js";
import {initMediaLocation, MediaLocation} from "./MediaLocationModel.js";
import {initMediaItem, MediaItem} from "./MediaItemModel.js";
import {initMediaLabel, MediaLabel} from "./MediaLabelModel.js";
import {initMediaGlossary, MediaGlossary} from "./MediaGlossaryModel.js";

export async function initMedia(db) {
    initMediaLabel(db);
    initMediaGlossary(db);
    initMediaClassification(db);
    initMediaLocation(db);
    initMediaItem(db);

    MediaItem.hasMany(MediaClassification);
    MediaClassification.belongsTo(MediaItem);

    MediaItem.hasOne(MediaLocation);
    MediaLocation.belongsTo(MediaItem);

    MediaClassification.hasMany(MediaLabel);
    MediaLabel.belongsTo(MediaClassification);

    MediaClassification.hasMany(MediaGlossary);
    MediaGlossary.belongsTo(MediaClassification);
}

export async function insertTestMediaItem() {
    let fileName = 'IMG_20200722_203422.jpg';
    if (!await MediaItem.findOne({where: {fileName}})) {
        let item = await MediaItem.create({
            type: 'image',
            subType: 'none',
            fileName,
            width: 1920,
            height: 1080,
            durationMs: null,
            bytes: 1245000000, //1.2GB
            createDate: new Date(),
            exif: {
                Make: "OnePlus",
                Model: "OnePlus 6T",
                Orientation: 8,
                ComponentsConfiguration: [1, 2, 3, 0],
            },
        });
        await MediaLocation.create({
            latitude: 50.123123,
            longitude: 5.1028931,
            altitude: 30.0,
            place: 'Sneek',
            country: 'The Netherlands',
            admin1: 'Sudwest Fryslan',
            admin2: 'Fryslan',
            MediaItemId: item.id,
        });
        let classification = await MediaClassification.create({
            confidence: Math.random(),
            MediaItemId: item.id,
        });
        for (let i = 0; i < 3; i++)
            await MediaLabel.create({
                text: 'cat',
                MediaClassificationId: classification.id
            });
        for (let i = 0; i < 3; i++)
            await MediaGlossary.create({
                text: 'A cat is an animal.',
                MediaClassificationId: classification.id
            });
    }

    let freshItem = await MediaItem.findOne({
        where: {fileName},
        include: [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation}
        ],
    });
    console.log("Created test media item", freshItem.toJSON());
}
