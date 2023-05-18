import {initClassification, Classification} from "./ClassificationModel.js";
import {initLocation, Location} from "./LocationModel.js";
import {initMedia, Media} from "./MediaModel.js";
import {initLabel, Label} from "./LabelModel.js";
import {initGlossary, Glossary} from "./GlossaryModel.js";
import sequelize from 'sequelize';
import {initPlace, Place} from "./PlaceModel.js";
import {initSuggestion, Suggestion} from "./SuggestionModel.js";
import Database from "../Database.js";
import {checkFileExists, getToken, months} from "../../utils.js";
import {dateToString, updatePhotoDate, updateVideoDate} from "../../modules/photos/exif.js";
import path from "path";
import config from '../../config.js'
import WordNet from "node-wordnet";
import {initBlocked, Blocked} from "./BlockedModel.js";
import fs from "fs";
import {getPaths, processMedia, uploadDir, zipDir} from "../../modules/photos/watchAndSynchonize.js";
import {filenameToDate} from "exif-date-fix";
import util from "util";
import archiver from "archiver";
import Clog from '../../Clog.js'
import {initLog, Log} from "./LogModel.js";
import {initLogSession, LogSession} from "./LogSessionModel.js";
import {Album, initAlbum} from "./AlbumModel.js";

const console = new Clog('mediaUtils');

const wordnet = new WordNet();
const {Op} = sequelize;

export async function initTables(db) {
    await db.query(`CREATE EXTENSION IF NOT EXISTS pg_trgm`);

    initLabel(db);
    initGlossary(db);
    initClassification(db);
    initLocation(db);
    initMedia(db);
    initPlace(db);
    initSuggestion(db);
    initBlocked(db);
    initLog(db);
    initLogSession(db);
    initAlbum(db);

    Media.hasMany(Classification, {onDelete: 'CASCADE'});
    Classification.belongsTo(Media);

    Media.hasOne(Location, {onDelete: 'CASCADE'});
    Location.belongsTo(Media);

    Location.hasMany(Place, {onDelete: 'CASCADE'});
    Place.belongsTo(Location);

    Classification.hasMany(Label, {onDelete: 'CASCADE'});
    Label.belongsTo(Classification);

    Classification.hasMany(Glossary, {onDelete: 'CASCADE'});
    Glossary.belongsTo(Classification);

    LogSession.hasMany(Log, {onDelete: 'CASCADE', foreignKey: {allowNull: false,}});
    Log.belongsTo(LogSession);

    const AlbumMedia = db.define('AlbumMedia', {
        id: {
            type: sequelize.INTEGER,
            primaryKey: true,
            autoIncrement: true
        },
    });

    Media.belongsToMany(Album, {through: {model: AlbumMedia, unique: false}});
    Album.belongsToMany(Media, {through: {model: AlbumMedia, unique: false}});
}

export async function injectAlbumCounts(albums) {
    let counts = await Promise.all(albums.map(album => Database.db.query(`
                        select count(*)
                        from "AlbumMedia"
                        where "AlbumId" = $1
                    `, {
        bind: [album.id],
        type: sequelize.QueryTypes.SELECT,
    })));
    let result = [];
    for (let i = 0; i < albums.length; i++) {
        result.push({
            ...albums[i].toJSON(),
            count: counts[i]?.[0]?.count,
        });
    }
    return result;
}

export async function getAlbums() {
    return await Database.db.query(`
        WITH summary AS (
            SELECT am."AlbumId",
                   count,
                   am."MediumId",
                   ROW_NUMBER() OVER (PARTITION BY am."AlbumId") AS rank
            FROM "AlbumMedia" am
                     inner join (select "AlbumId", count(*) as count
                                 from "AlbumMedia"
                                 group by "AlbumId") as test on test."AlbumId" = am."AlbumId")
        SELECT summary.count, A.id, "MediumId", "name", "createdAt", "updatedAt"
        FROM summary
                 right JOIN "Albums" A on A.id = "AlbumId"
        WHERE rank = greatest(1, summary.count / 2)
           or "MediumId" IS NULL
        order by "updatedAt" desc;
    `, {
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function deleteOldLogs(cutoffDate = null) {
    const day = 1000 * 60 * 60 * 24;
    cutoffDate ??= new Date(Date.now() - day * 7);
    console.log("Deleting log sessions older than", cutoffDate);
    await LogSession.destroy({
        where: {
            createdAt: {
                [Op.lte]: cutoffDate,
            },
        },
    });
    console.log("Done deleting log");
}

export async function deleteOldZips(cutoffDate = null) {
    console.log("starting delete old zips")
    const day = 1000 * 60 * 60 * 24;
    cutoffDate ??= new Date(Date.now() - day * 7);
    console.log("Deleting zip downloads older than", cutoffDate);
    let files = await fs.promises.readdir(zipDir);
    let stats = await Promise.all(files.map(f => fs.promises.stat(path.join(zipDir, f))));
    for (let i = 0; i < stats.length; i++) {
        let date = new Date(stats[i].ctime);
        if (date < cutoffDate) {
            console.log(`Deleting old zip download: "${files[i]}"`)
            await fs.promises.unlink(path.join(zipDir, files[i]));
        }
    }
}

export async function uploadFile(file) {
    let destination = path.join(uploadDir, file.name);
    try {
        let fileExists = await checkFileExists(destination);
        let filePath = path.relative(config.media, destination);
        let mediaMedia = await Media.findOne({where: {filePath}});
        if (mediaMedia) {
            return {success: false, error: 'file already exists!', id: mediaMedia.id};
        }
        if (!fileExists) {
            await util.promisify(file.mv)(destination);
        }
        let blocked = await Blocked.findOne({where: {filePath}});
        if (blocked)
            await blocked.destroy();
        let id = await processMedia(destination);
        if (id === false) {
            return {success: false, error: 'media processing error'};
        } else {
            return {success: true, id};
        }
    } catch (e) {
        return {success: false, error: e.message};
    }
}

export function getZipPath(zipId) {
    return path.join(zipDir, zipId + '.zip');
}

export async function createZip(files) {
    let zipId = await getToken();
    let zipPath = getZipPath(zipId);
    const output = fs.createWriteStream(zipPath);
    const archive = archiver('zip', {
        zlib: {level: 1}
    });
    output.on('close', () => {
        console.log(archive.pointer() + ' total bytes');
        console.log('archiver has been finalized and the output file descriptor has closed.');
    });
    output.on('end', () => console.log('Data has been drained'));
    archive.on('warning', err => {
        console.warn(err);
    });
    archive.on('error', err => {
        throw err;
    });
    archive.pipe(output);

    let addedFiles = [];
    for (let file of files) {
        let name = path.basename(file);
        let i = 0;
        while (addedFiles.includes(name)) {
            let extName = path.extname(name);
            let nameWithoutExt = name.substr(name.length - extName.length);
            name = `${nameWithoutExt}(${i})${extName}`
        }
        addedFiles.push(name);
        archive.file(file, {name});
    }
    await archive.finalize();
    return zipId;
}

export async function autoFixDate(id) {
    let item = await Media.findOne({where: {id}});
    if (item === null) return {success: false, code: 404};
    let dateString = filenameToDate(item.filename);
    if (dateString !== null) {
        return await changeMediaDate(item, dateString);
    } else {
        return false;
    }
}

export async function dropAndReprocess(id, filePath, transaction = null) {
    const config = async transaction => {
        let mediaAlbums = await Album.findAll({
            include: {
                model: Media,
                where: {id},
            },
            transaction,
        });
        await dropMedia(id, transaction);
        let newId = await processMedia(filePath, 2, transaction);
        if (id === false)
            return false;
        for (let album of mediaAlbums) {
            await album.addMedium(
                await Media.findOne({
                    where: {id: newId},
                    transaction,
                }),
                {transaction},
            );
        }
        return newId;
    };
    if (transaction === null) {
        let result = await Database.db.transaction({}, config);
        console.log(result);
        return result;
    } else {
        let result = await config(transaction);
        console.log(result);
        return result;
    }
}

export async function reprocess(id) {
    let item = await Media.findOne({where: {id}});
    if (item === null) return {success: false, code: 404};

    let filePath = path.resolve(path.join(config.media, item.filePath));

    try {
        let newId = await dropAndReprocess(item.id, filePath);
        return {success: true, id: newId};
    } catch (e) {
        console.warn("reprocess failed", e.message);
        return {success: false, code: 500};
    }
}

export async function deleteFile(id) {
    let item = await Media.findOne({where: {id}});
    if (item === null) return {success: false, code: 404};

    try {
        let filePath = path.resolve(path.join(config.media, item.filePath));
        await dropMedia(id);
        if (await checkFileExists(filePath))
            await fs.promises.unlink(filePath);
        let files = getPaths(id);
        for (let key in files)
            if (files.hasOwnProperty(key))
                if (await checkFileExists(files[key])) {
                    console.log("Deleting", files[key])
                    await fs.promises.unlink(files[key])
                }
        if (!await Blocked.findOne({where: {filePath: item.filePath}}))
            await Blocked.create({
                type: item.type,
                filePath: item.filePath,
                reason: 'deleted',
                id: await getToken(),
            });
        console.log("Deleted item", filePath);
        return {success: true};
    } catch (e) {
        console.warn("Delete failed", e);
        return {success: false};
    }
}

export async function getBoundingBox(place) {
    return await Database.db.query(`
        select max(latitude) as maxlat, min(latitude) as minlat, max(longitude) as maxlng, min(longitude) as minlng
        from "Locations"
            inner join "Places" MP on "Locations".id = MP."LocationId"
        where text = $1
    `, {
        bind: [place],
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function getGlossary(label) {
    try {
        label = label.replace(/ /g, '_');
        let words = await wordnet.lookupAsync(label);
        let results = words.filter(w => w.synonyms.includes(label) && w.pos === 'n');
        let glossaries = await Promise.all(
            results.map(r => Glossary.findOne({where: {text: r.gloss.trim()}}))
        ).then(r => r.filter(w => w !== null));
        if (glossaries.length === 0) return {isLabel: false, glossary: null};
        let glossary = glossaries[0].text;
        const capitalize = c => c.substr(0, 1).toUpperCase() + c.substr(1);
        glossary = glossary.split('.').map(capitalize).join('.');
        return {isLabel: true, glossary};
    } catch (e) {
        return {isLabel: false, glossary: null};
    }
}

export async function changeMediaDate(item, newDateString) {
    if (newDateString === null || item === null) return false;
    let newDate = new Date(newDateString);

    let suggestions = item.createDate !== null ? getDateSuggestions(item.createDate) : [];
    let newSuggestions = getDateSuggestions(newDate);

    await Database.db.transaction({}, async transaction => {
        await Promise.all(suggestions.map(o => removeSuggestion(o, transaction)))
        await item.update({
            createDate: newDate,
            createDateString: newDateString,
        }, {transaction});
        await Promise.all(newSuggestions.map(o => addSuggestion(o, transaction)));
    });
    try {
        if (item.type === 'image') {
            await updatePhotoDate(path.join(config.media, item.filePath), newDate);
        } else {
            await updateVideoDate(path.join(config.media, item.filePath), newDate);
        }
    } catch (e) {
        console.log(`Couldn't update file (${item.filename}) date to ${newDate}\n`, e);
    }
    return true;
}

export async function getMonthPhotos(year, month) {
    return await Database.db.query(`
        select id, type, "subType", width, height, "createDateString", "durationMs"
        from "Media"
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
        select extract(year from "createDate")::INT  as year,
               extract(month from "createDate")::INT as month,
               count(*)::INT                         as count
        from "Media"
        where "createDate" is not null
        group by year, month
        order by year desc, month desc;
    `, {
        type: sequelize.QueryTypes.SELECT,
    });
}

export async function getRandomLocations(limit = 50) {
    return await Database.db.query(`
        select distinct on (text) text, "MediumId"
        from "Locations"
        inner join "Places" MP on "Locations".id = MP."LocationId"
        where text in (select text
        from (select text, count(text)::FLOAT / (select count(*) from "Places") * 10 + random() as count
        from "Places"
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
        select distinct on (text) text, "MediumId", confidence
        from "Classifications"
                 inner join "Labels" ML on "Classifications".id = ML."ClassificationId"
        where confidence = 1
          and text in (
            select text
            from (
                     select text, count(text)::FLOAT / (select count(*) from "Labels") * 25 + random() as count
                     from "Labels"
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
    from "Media", to_tsquery('english', $1) query
    where query @@ vector
    order by "createDate" desc
        ${limit ? `limit = $2` : ''}`,
        {
            model: Media,
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
            {model: Classification, include: [Label, Glossary]},
            {model: Location}
        ]
    return await Media.findAll(options);
}

export async function getMediaByFilename(filename) {
    return await Media.findOne({
        where: {filename},
        include: [
            {model: Classification, include: [Label, Glossary]},
            {model: Location, include: [Place]},
        ],
    });
}

export async function getMediaById(id) {
    return await Media.findOne({
        where: {id},
        include: [
            {model: Classification, include: [Label, Glossary]},
            {model: Location, include: [Place]},
        ],
    });
}

export async function getUniqueId() {
    let id;
    do {
        id = await getToken(16);
    } while (await Media.findOne({where: {id}}));
    return id;
}

export async function getPhotosForMonth(month) {
    return await Database.db.query(`
        select id, type, "subType", width, height, "createDateString", "durationMs"
        from "Media"
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
        from "Media"
        where extract(month from "createDate") = $2
          and extract(day from "createDate") = $1
        order by "createDate" desc;
    `, {
        bind: [day, month],
        type: sequelize.QueryTypes.SELECT,
    });
}

export function getDateSuggestions(date) {
    if (date === null)
        return [];
    let suggestions = [];
    let day = date.getDate();
    let month = months[date.getMonth()];
    let year = date.getFullYear().toString();
    suggestions.push({type: 'date', text: month});
    suggestions.push({type: 'date', text: year});
    suggestions.push({type: 'date', text: `${month} ${year}`});
    suggestions.push({type: 'date', text: `${day} ${month}`});
    suggestions.push({type: 'date', text: `${day} ${month} ${year}`});
    return suggestions;
}

export function getClassificationSuggestions(classifications) {
    if (!Array.isArray(classifications))
        return [];
    return classifications
        .flatMap(c => c.Labels ?? [])
        .map(p => ({text: p.text, type: 'label'}))
}

export function getPlacesSuggestions(places) {
    if (!Array.isArray(places))
        return [];
    return places.map(p => ({text: p.text, type: 'place'}))
}

export async function dropMedia(id, transaction = null) {
    let spreadTransaction = transaction ? {transaction} : {};
    let item = await Media.findOne({
        where: {id},
        include: [
            {model: Classification, include: [Label]},
            {model: Location, include: [Place]},
        ],
        ...spreadTransaction,
    });
    // Remove from album
    await Database.db.query(`
        delete
        from "AlbumMedia"
        where "MediumId" = $1
    `, {
        bind: [id],
        ...spreadTransaction,
        type: sequelize.QueryTypes.DELETE,
    });
    if (item === null)
        return false;
    let suggestions = [];
    suggestions.push(...getPlacesSuggestions(item.Location?.Places));
    suggestions.push(...getClassificationSuggestions(item.Classifications));
    suggestions.push(...getDateSuggestions(item.createDate));
    await Promise.all(suggestions.map(o => removeSuggestion(o, transaction)));

    await item.destroy({...spreadTransaction});
}

export async function removeSuggestion(obj, transaction) {
    let spreadTransaction = transaction ? {transaction} : {};
    if (typeof obj.type !== 'string' || typeof obj.text !== 'string') {
        console.warn("Can't remove suggestion", obj);
        return;
    }

    const {text, type} = obj;
    let suggestion = await Suggestion.findOne({where: {type, text}, ...spreadTransaction});
    if (suggestion === null)
        return;
    if (suggestion.count === 1) {
        await suggestion.destroy({...spreadTransaction});
    } else {
        suggestion.count--;
        await suggestion.save({...spreadTransaction});
    }
}

export async function addSuggestion(obj, transaction) {
    if (typeof obj.type !== 'string' || typeof obj.text !== 'string') {
        console.warn("Can't add suggestion", obj);
        return;
    }
    const {text, type} = obj;
    let [item, created] = await Suggestion.findOrCreate({
        where: {text}, defaults: {
            type,
            data: obj.data ?? null,
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
    let monthName = months[month - 1];
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
 *     width,height,durationMs?,bytes,createDateString?:string,exif,
 *     location?: {latitude,longitude,altitude?,place?,country?,admin: string[]?},
 *     classifications?: {confidence: number, labels: string[], glossaries: string[]}[],
 * }} data Media data
 * @param transaction? Sequelize transaction
 * @returns {Promise<void>}
 */
export async function insertMedia(data, transaction = null) {
    let [classA, classB, classC] = data.classifications ?
        data.classifications.sort((a, b) => b.confidence - a.confidence) :
        [null, null, null];

    try {
        const insert = async transaction => {
            data.createDate = new Date(data.createDateString);
            if (typeof data.createDateString !== 'string') {
                console.warn("FOUT")
            }
            let item = await Media.create({
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
                `update "Media"
                    set vector = "vectorA" || "vectorB" || "vectorC"
                    where id = '${data.id}'`
                , {transaction});

            // Add suggestions to db
            const places = [
                data.location?.place, data.location?.country,
                ...(data.location?.admin ?? [])
            ].flat().filter(n => n !== null && n !== undefined).map(p => ({text: p, type: 'place'}));
            const labels = [
                classA?.labels, classB?.labels, classC?.labels
            ].flat().filter(n => n !== null && n !== undefined).map(p => ({text: p.text, type: 'label'}));

            let dates = getDateSuggestions(data.createDate);
            try {
                await Promise.all([...places, ...labels, ...dates].map(o => addSuggestion(o, transaction)));
            } catch (e) {
            }

            if (data.location) {
                let locMedia = await Location.create({
                    latitude: data.location.latitude,
                    longitude: data.location.longitude,
                    altitude: data.location.altitude,
                    MediumId: item.id,
                }, {transaction});
                let places = [
                    {type: 'place', text: data.location.place},
                    {type: 'country', text: data.location.country},
                    ...data.location.admin.map((a, i) => ({
                        type: `admin${i + 1}`
                        , text: a
                    })),
                ];
                await Place.bulkCreate(places.map(({text, type}) => ({
                        text,
                        type,
                        isCode: text.match(/[0-9]+/g) !== null,
                        LocationId: locMedia.id,
                    })), {transaction}
                )
            }
            if (data.classifications)
                for (let {confidence, labels, glossaries} of data.classifications) {
                    if (item.id === null)
                        console.warn("FOUT 2")
                    let classification = await Classification.create({
                        confidence,
                        MediumId: item.id,
                    }, {transaction});
                    await Label.bulkCreate(labels.map(label => ({
                        ...label,
                        ClassificationId: classification.id
                    })), {transaction});
                    await Glossary.bulkCreate(glossaries.map(glossary => ({
                        ...glossary,
                        ClassificationId: classification.id
                    })), {transaction});
                }
        };
        if (transaction === null)
            await Database.db.transaction({}, insert);
        else
            await insert(transaction);
        console.log(`inserted ${data.filename}`);
        // transaction has been committed. Do something after the commit if required.
    } catch (err) {
        console.warn("Media insert ERROR", err);
        throw new Error(err);
        // do something with the err.
    }
}

export async function setDefaultAlbumCover(album) {
    let newCover = await Database.db.query(`
                        select "MediumId"
                        from "AlbumMedia"
                        where "AlbumId" = $1
                        offset (
                            select count(*) / 2
                            from "AlbumMedia"
                            where "AlbumId" = $1
                        ) limit 1
                    `, {bind: [album.id], type: sequelize.QueryTypes.SELECT});
    await album.update({
        cover: newCover?.[0]?.MediumId ?? '',
    });
    return album;
}

async function swapWidthAndHeightOnPortraitPhotos() {
    await Database.db.query(`
        update "Media"
        set width=height,
            height=width
        where id in (
            select id
            from "Media"
            where (exif -> 'Orientation')::INT > 4
        )
    `, {type: sequelize.QueryTypes.UPDATE});
}
