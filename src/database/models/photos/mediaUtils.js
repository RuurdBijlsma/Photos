import {initMediaClassification, MediaClassification} from "./MediaClassificationModel.js";
import {initMediaLocation, MediaLocation} from "./MediaLocationModel.js";
import {initMediaItem, MediaItem} from "./MediaItemModel.js";
import {initMediaLabel, MediaLabel} from "./MediaLabelModel.js";
import {initMediaGlossary, MediaGlossary} from "./MediaGlossaryModel.js";
import Utils from "../../../Utils.js";
import path from "path";
import sequelize from 'sequelize';

const {Op} = sequelize;

export async function initMedia(db) {
    initMediaLabel(db);
    initMediaGlossary(db);
    initMediaClassification(db);
    initMediaLocation(db);
    initMediaItem(db);

    MediaItem.hasMany(MediaClassification, {onDelete: 'CASCADE'});
    MediaClassification.belongsTo(MediaItem);

    MediaItem.hasOne(MediaLocation, {onDelete: 'CASCADE'});
    MediaLocation.belongsTo(MediaItem);

    MediaClassification.hasMany(MediaLabel, {onDelete: 'CASCADE'});
    MediaLabel.belongsTo(MediaClassification);

    MediaClassification.hasMany(MediaGlossary, {onDelete: 'CASCADE'});
    MediaGlossary.belongsTo(MediaClassification);
}

export async function searchMedia({
                                      query,
                                      limit = false,
                                      include = false,
                                      attributes = false,
                                  }) {
    let options = {
        where: {
            vector: {[Op.match]: sequelize.fn('to_tsquery', query)}
        },
    }
    if (attributes)
        options.attributes = attributes;
    if (limit)
        options.limit = limit;
    if (include)
        options.include = [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation}
        ]
    return await MediaItem.findAll(options);
}

export async function getMediaByFilename(filename) {
    return await MediaItem.findOne({
        where: {filename},
        include: [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation}
        ],
    });
}

export async function getMediaById(id) {
    return await MediaItem.findOne({
        where: {id},
        include: [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation}
        ],
    });
}

/**
 * @param {{
 *     type,subType,filename,filePath,smallThumbPath,bigThumbPath,webmPath,
 *     width,height,durationMs?,bytes,createDate?,exif,
 *     location?: {latitude,longitude,altitude?,place?,country?,admin1?,admin2?,admin3?,admin4?},
 *     classifications?: {confidence: number, labels: string[], glossaries: string[]}[],
 * }} data MediaItem data
 * @returns {Promise<void>}
 */
export async function insertMediaItem(data) {
    let id;
    do {
        id = await Utils.getToken(16);
    } while (await MediaItem.findOne({where: {id}}));

    const toVector = (weight, ...items) =>
        sequelize.fn('setweight',
            sequelize.fn('to_tsvector', 'english',
                items.filter(n => n !== null && n !== undefined).join(' ')
            ),
            weight
        );
    let [classA, classB, classC] = data.classifications.sort((a, b) => b.confidence - a.confidence);
    let item = await MediaItem.create({
        id,
        vectorA: toVector('A',
            ...classA.labels,
            data.location?.place,
            data.location?.country,
        ),
        vectorB: toVector('B',
            data.filename, ...classB.labels,
            data.location?.admin1,
            data.location?.admin2,
            data.location?.admin3,
            data.location?.admin4,
        ),
        vectorC: toVector('C',
            ...classC.labels, ...classB.glossaries, ...classA.glossaries, classC.glossaries
        ),
        ...data
    });
    await MediaItem.sequelize.query(
        `update "MediaItems"
         set vector = setweight("vectorA", 'A') || setweight("vectorB", 'B') || setweight("vectorC", 'C')
         where id = '${id}'`
    );
    if (data.location)
        await MediaLocation.create({
            ...data.location,
            MediaItemId: item.id,
        });
    if (data.classifications)
        for (let {confidence, labels, glossaries} of data.classifications) {
            let classification = await MediaClassification.create({
                confidence,
                MediaItemId: item.id,
            });
            for (let text of labels)
                await MediaLabel.create({
                    text,
                    MediaClassificationId: classification.id
                });
            for (let text of glossaries)
                await MediaGlossary.create({
                    text,
                    MediaClassificationId: classification.id
                });
        }
}

export async function removeTestMediaItem() {
    let filename = 'IMG_20200722_203422.jpg';
    let item = await MediaItem.findOne({where: {filename}});
    await item.destroy();
    console.log();
}

export async function insertTestMediaItem() {
    let filePath = 'IMG_20200722_203422.jpg';
    let filename = path.basename(filePath);
    if (!await MediaItem.findOne({where: {filename}})) {
        await insertMediaItem({
            type: 'image',
            subType: 'none',
            filename,
            filePath,
            bigThumbPath: '/bigt.webp',
            smallThumbPath: 'smallt.webp',
            webmPath: 'vid.webm',
            width: 1280,
            height: 720,
            bytes: 128038102,
            createDate: new Date(),
            exif: {
                Make: 'OnePlus',
            },
            classifications: [
                {
                    labels: ['cat', 'animal'],
                    glossaries: ['domestic animal', 'animal description here'],
                    confidence: 1,
                }
            ],
            location: {
                longitude: 50,
                latitude: 5,
                altitude: 0,
                country: 'TempLand',
                place: 'TempTown',
                admin1: 'Temp District',
                admin2: 'Temp Region',
            }
        })
    }

    let freshItem = await getMediaByFilename(filename);
    console.log("Created test media item", freshItem.toJSON());
}
