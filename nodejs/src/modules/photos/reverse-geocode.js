import geocoder from "local-reverse-geocoder";
import lookup from "country-code-lookup";
import config from '../../config.js'
import Log from '../../Log.js'

const console = new Log('geocode');

if (config.skipGeocode)
    console.warn("SKIPPING GEOCODING");

async function init() {
    if (config.skipGeocode)
        return;

    return new Promise(resolve => {
        geocoder.init({
            load: {
                admin1: true,
                admin2: true,
                admin3And4: config.geocodeAdmin3and4,
                alternateNames: false,
            },
            dumpDirectory: './res/geonames'
        }, () => {
            resolve();
        });
    })
}

let ready = init();

export default async function geocode({latitude, longitude}) {
    if (config.skipGeocode)
        return {place: 'Temp', country: 'Templand', admin: ['temi', 'temp2', 'ajsmdp8']}; // to speed up testing

    const point = {latitude, longitude};
    // console.time("GEOCODE " + JSON.stringify(point));

    await ready;

    return new Promise((resolve, reject) => {
        const maxResults = 1;
        geocoder.lookUp(point, maxResults,
            (err, res) => {
                if (err)
                    return reject("Geocode error for", point);
                let [[{name: place, countryCode: country, admin1Code, admin2Code, admin3Code, admin4Code}]] = res;
                let geocodeData = {
                    place,
                    country: lookup.byIso(country).country,
                    admin: [admin1Code, admin2Code]
                };
                if (config.geocodeAdmin3and4)
                    geocodeData.admin.push(admin3Code, admin4Code);
                geocodeData.admin = geocodeData.admin
                    .filter(a => a)
                    .map(a => a.hasOwnProperty('name') ? a.name : a)
                    .reverse();
                resolve(geocodeData);
            });
    })
}

// geocode({latitude: 36.14745277777778, longitude: -5.355142777777777}).then(c => {
//     console.log(c)
// })
