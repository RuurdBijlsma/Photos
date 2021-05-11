import geocoder from "local-reverse-geocoder";
import lookup from "country-code-lookup";

async function init() {
    return; // To speed up testing

    return new Promise(resolve => {
        geocoder.init({
            load: {
                admin1: true,
                admin2: true,
                admin3And4: true,
                alternateNames: false,
            },
            dumpDirectory: './res/photos/geonames'
        }, resolve);
    })
}

let ready = init();

export default async function geocode(point = {latitude: 48.45349, longitude: 9.09582}) {
    return {place: 'Temp', country: 'Templand', admin: ['Temp-west', 'TempoTown']}; // to speed up testing

    await ready;

    return new Promise((resolve, reject) => {
        const maxResults = 1;
        geocoder.lookUp(point, maxResults,
            (err, res) => {
                if (err)
                    return reject();
                let [[{name: place, countryCode: country, admin1Code, admin2Code, admin3Code, admin4Code}]] = res;
                let geocodeData = {
                    place,
                    country: lookup.byIso(country).country,
                    admin: [admin1Code, admin2Code, admin3Code, admin4Code]
                };
                geocodeData.admin = geocodeData.admin
                    .filter(a => a)
                    .map(a => a.name)
                    .reverse();
                resolve(geocodeData);
            });
    })
}

// geocode({latitude: 41.219915, longitude: 19.696557}).then(c => {
//     console.log(c)
// })
