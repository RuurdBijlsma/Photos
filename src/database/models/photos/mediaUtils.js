import {initMediaClassification, MediaClassification} from "./MediaClassificationModel.js";
import {initMediaLocation, MediaLocation} from "./MediaLocationModel.js";
import {initMediaItem, MediaItem} from "./MediaItemModel.js";
import {initMediaLabel, MediaLabel} from "./MediaLabelModel.js";
import {initMediaGlossary, MediaGlossary} from "./MediaGlossaryModel.js";
import Utils from "../../../Utils.js";
import path from "path";
import sequelize from 'sequelize';
import {initMediaPlace, MediaPlace} from "./MediaPlaceModel.js";
import {initMediaSuggestion, MediaSuggestion} from "./MediaSuggestionModel.js";
import Database from "../../Database.js";

const {Op} = sequelize;

export async function initMedia(db) {
    await db.query(`CREATE EXTENSION IF NOT EXISTS pg_trgm`);

    initMediaLabel(db);
    initMediaGlossary(db);
    initMediaClassification(db);
    initMediaLocation(db);
    initMediaItem(db);
    initMediaPlace(db);
    initMediaSuggestion(db);

    MediaItem.hasMany(MediaClassification, {onDelete: 'CASCADE'});
    MediaClassification.belongsTo(MediaItem);

    MediaItem.hasOne(MediaLocation, {onDelete: 'CASCADE'});
    MediaLocation.belongsTo(MediaItem);

    MediaLocation.hasMany(MediaPlace, {onDelete: 'CASCADE'});
    MediaPlace.belongsTo(MediaLocation);

    MediaClassification.hasMany(MediaLabel, {onDelete: 'CASCADE'});
    MediaLabel.belongsTo(MediaClassification);

    MediaClassification.hasMany(MediaGlossary, {onDelete: 'CASCADE'});
    MediaGlossary.belongsTo(MediaClassification);
}

export async function getMonthPhotos(year, month) {
    return await Database.db.query(`
        select id, type, "subType", width, height, "createDate", "durationMs"
        from "MediaItems"
        where extract(month from "createDate") = $1
          and extract(year from "createDate") = $2
        order by "createDate" desc;
    `, {
        bind: [month, year],
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function getPhotoMonths() {
    return await Database.db.query(`
        select extract(year from "createDate")                       as year,
               extract(month from "createDate")                      as month,
               count(*)::INT                                         as count
        from "MediaItems"
        where "createDate" is not null
        group by year, month
        order by year desc, month desc;
    `, {
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function getRandomLocations(limit = 50) {
    return await Database.db.query(`
        select distinct on (text) text, "MediaItemId"
        from "MediaLocations"
        inner join "MediaPlaces" MP on "MediaLocations".id = MP."MediaLocationId"
        where text in (select text
        from (select text, count(text)::FLOAT / (select count(*) from "MediaPlaces") * 10 + random() as count
        from "MediaPlaces"
        where not "isCode"
        group by text
        order by count desc
        limit $1) as counttable)
    `, {
            bind: [limit],
            type: sequelize.QueryTypes.SELECT,
        }
    );
}

export async function getRandomLabels(limit = 50) {
    return await Database.db.query(`
    select distinct on (text) text, "MediaItemId"
    from "MediaClassifications"
    inner join "MediaLabels" ML on "MediaClassifications".id = ML."MediaClassificationId"
    where text in (
    select text
    from (
    select text, count(text)::FLOAT / (select count(*) from "MediaLabels") * 25 + random() as count
    from "MediaLabels"
    where level <= 2
    group by text
    order by count desc
    limit $1
    ) as counttable)
    `, {
            bind: [limit],
            type: sequelize.QueryTypes.SELECT,
        }
    );
}

export async function searchMediaRanked({query, limit = false, includedFields}) {
    query = query.replace(/ /g, '&');
    let bind = [query];
    if (limit)
        bind.push(limit);
    return await Database.db.query(
        `select ${includedFields.map(f=>`"${f}"`).join(', ')}, ts_rank_cd(vector, query) as rank
    from "MediaItems", to_tsquery('english', $1) query
    where query @@ vector
    order by rank desc
        ${limit ? `limit = $2` : ''}`,
        {
            model: MediaItem,
            mapToModel: true,
            bind,
            type: sequelize.QueryTypes.SELECT,
        }
    );
}

export async function searchMedia({
                                      query,
                                      limit = false,
                                      include = false,
                                      attributes = false,
                                  }) {
    let options = {
        where: {
            vector: {[Op.match]: sequelize.fn('to_tsquery', 'english', query)}
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
            {model: MediaLocation, include: [MediaPlace]},
        ],
    });
}

export async function getMediaById(id) {
    return await MediaItem.findOne({
        where: {id},
        include: [
            {model: MediaClassification, include: [MediaLabel, MediaGlossary]},
            {model: MediaLocation, include: [MediaPlace]},
        ],
    });
}

export async function getUniqueId() {
    let id;
    do {
        id = await Utils.getToken(16);
    } while (await MediaItem.findOne({where: {id}}));
    return id;
}

export async function getPhotosForMonth(month) {
    return await Database.db.query(`
        select id, type, "subType", width, height, "createDate", "durationMs"
        from "MediaItems"
        where extract(month from "createDate") = $1
        order by "createDate" desc;
    `, {
        bind: [month],
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function getPhotosPerDayMonth(day, month) {
    return await Database.db.query(`
        select *
        from "MediaItems"
        where extract(month from "createDate") = $2
          and extract(day from "createDate") = $1
        order by "createDate" desc;
    `, {
        bind: [day, month],
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function addSuggestion(obj, transaction) {
    if (typeof obj.type !== 'string' || typeof obj.text !== 'string') {
        console.warn("Can't add suggestion", obj);
        return;
    }
    const {text, type} = obj;
    let [item, created] = await MediaSuggestion.findOrCreate({
        where: {text}, defaults: {
            type,
            vector: sequelize.fn('to_tsvector', 'english', text),
            count: 1,
        },
        transaction,
    });
    if (!created) {
        item.count++;
        await item.save({transaction});
    }
}

export function dateToWords(date) {
    if (date === null || date === undefined)
        return [];

    let year = date.getFullYear();
    let month = date.getMonth() + 1;
    let day = date.getDate();
    let monthName = Utils.months[month - 1];
    let shortMonthName = monthName.substr(0, 3);
    return [day, month, shortMonthName, monthName, year];
}

export const toVector = (weight, ...items) =>
    sequelize.fn('setweight',
        sequelize.fn('to_tsvector', 'english',
            items.filter(n => n !== null && n !== undefined).join(' ')
        ),
        weight
    );
export const toText = a => Array.isArray(a) ? a.map(b => b.text) : [a];

/**
 * @param {{
 *     id,type,subType,filename,filePath,
 *     width,height,durationMs?,bytes,createDate?:Date,exif,
 *     location?: {latitude,longitude,altitude?,place?,country?,admin: string[]?},
 *     classifications?: {confidence: number, labels: string[], glossaries: string[]}[],
 * }} data MediaItem data
 * @returns {Promise<void>}
 */
export async function insertMediaItem(data) {
    let [classA, classB, classC] = data.classifications ?
        data.classifications.sort((a, b) => b.confidence - a.confidence) :
        [null, null, null];

    try {
        await Database.db.transaction({}, async transaction => {
            let item = await MediaItem.create({
                vectorA: toVector('A',
                    ...toText(classA?.labels),
                    ...dateToWords(data.createDate),
                    data.location?.place,
                    data.location?.country,
                ),
                vectorB: toVector('B',
                    data.filename,
                    ...toText(classB?.labels),
                    ...(data.location?.admin ?? []),
                    data.subType === 'none' ? null : data.subType,
                ),
                vectorC: toVector('C',
                    ...toText(classC?.labels),
                    ...toText(classA?.glossaries),
                    ...toText(classB?.glossaries),
                    ...toText(classC?.glossaries),
                    data.type,
                ),
                ...data,
            }, {transaction});
            await Database.db.query(
                `update "MediaItems"
                    set vector = "vectorA" || "vectorB" || "vectorC"
                    where id = '${data.id}'`
                , {transaction});

            // Add suggestions to db
            const places = [
                data.location?.place, data.location?.country,
                ...(data.location?.admin ?? [])
            ].flat().filter(n => n !== null && n !== undefined).map(p => ({text: p, type: 'place'}));
            const labels = [
                ...classA?.labels, classB?.labels, classC?.labels
            ].flat().filter(n => n !== null && n !== undefined).map(p => ({text: p.text, type: 'label'}));

            let dates = [];
            if (data.createDate !== null) {
                let date = data.createDate;
                let day = date.getDate();
                let month = Utils.months[date.getMonth()];
                let year = date.getFullYear().toString();
                dates.push({type: 'date', text: month});
                dates.push({type: 'date', text: year});
                dates.push({type: 'date', text: `${month} ${year}`});
                dates.push({type: 'date', text: `${day} ${month}`});
                dates.push({type: 'date', text: `${day} ${month} ${year}`});
            }

            try {
                await Promise.all([...places, ...labels, ...dates].map(o => addSuggestion(o, transaction)));
            } catch (e) {
            }

            if (data.location) {
                let locItem = await MediaLocation.create({
                    latitude: data.location.latitude,
                    longitude: data.location.longitude,
                    altitude: data.location.altitude,
                    MediaItemId: item.id,
                }, {transaction});
                let places = [
                    {type: 'place', text: data.location.place},
                    {type: 'country', text: data.location.country},
                    ...data.location.admin.map((a, i) => ({
                        type: `admin${i + 1}`
                        , text: a
                    })),
                ];
                await MediaPlace.bulkCreate(places.map(({text, type}) => ({
                        text,
                        type,
                        isCode: text.match(/[0-9]+/g) !== null,
                        MediaLocationId: locItem.id,
                    })), {transaction}
                )
            }
            if (data.classifications)
                for (let {confidence, labels, glossaries} of data.classifications) {
                    let classification = await MediaClassification.create({
                        confidence,
                        MediaItemId: item.id,
                    }, {transaction});
                    await MediaLabel.bulkCreate(labels.map(label => ({
                        ...label,
                        MediaClassificationId: classification.id
                    })), {transaction});
                    await MediaGlossary.bulkCreate(glossaries.map(glossary => ({
                        ...glossary,
                        MediaClassificationId: classification.id
                    })), {transaction});
                }
        });
        console.log(
            `inserted ${data.filename}`
        );
        // transaction has been committed. Do something after the commit if required.
    } catch (err) {
        console.warn("MediaItem insert ERROR", err);
        throw new Error(err);
        // do something with the err.
    }
}

export async function removeTestMediaItem() {
    let filename = 'IMG_20200722_203422.jpg';
    let item = await MediaItem.findOne({where: {filename}});
    await item.destroy();
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

async function swapWidthAndHeightOnPortraitPhotos() {
    await Database.db.query(`
        update "MediaItems"
        set width=height,
            height=width
        where id in (
            select id
            from "MediaItems"
            where (exif -> 'Orientation')::INT > 4
        )
    `, {type: sequelize.QueryTypes.UPDATE});
}
