use color_eyre::eyre::{Context, Result};
use common_types::ml_analysis::MLLlmClassification;
use language_model::LlamaClient;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::Path;
use tracing::warn;

#[derive(Deserialize, Default, Debug)]
struct BasicClassifyCaption {
    caption: String,
    main_subject: String,
    setting: String,
    search_term: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Deserialize, Default)]
struct ClassifyFlags {
    contains_pets: bool,
    contains_animals: bool,
    contains_vehicle: bool,
    famous_landmarks: bool,
    contains_people: bool,
    is_indoor: bool,
    depicts_food: bool,
    depicts_drink: bool,
    depicts_event: bool,
    contains_document: bool,
    depicts_landscape: bool,
    depicts_cityscape: bool,
    depicts_physical_activity: bool,
    contains_legible_text: bool,
}

#[derive(Deserialize, Default)]
struct ClassifyDetails {
    animal_name: Option<String>,
    food_name: Option<String>,
    drink_name: Option<String>,
    vehicle_type: Option<String>,
    event_type: Option<String>,
    landmark_name: Option<String>,
    activity_name: Option<String>,
    document_type: Option<String>,
    people_mood: Option<String>,
    photo_type: Option<String>,
    people_count: Option<i32>,
}

pub async fn get_llm_classification(
    llm: &LlamaClient,
    img_path: &Path,
) -> Result<MLLlmClassification> {
    let basic_info: BasicClassifyCaption =
        request_structured_data(llm, img_path, prompts::BASIC_INFO, schemas::basic_info()).await?;
    let flags: ClassifyFlags = request_structured_data(
        llm,
        img_path,
        prompts::CATEGORIZATION,
        schemas::categories(),
    )
    .await?;
    let details = get_detailed_analysis(llm, img_path, &flags).await?;
    let ocr_text = get_ocr(llm, flags.contains_legible_text, img_path).await?;
    Ok(map_to_final_result(basic_info, &flags, details, ocr_text))
}

async fn request_structured_data<T: DeserializeOwned + Default>(
    llm: &LlamaClient,
    file: &Path,
    prompt: &str,
    schema: Value,
) -> Result<T> {
    let response = llm
        .chat(prompt)
        .images(&[file])
        .schema(schema)
        .call()
        .await
        .wrap_err("LLM request failed")?;

    match serde_json::from_str::<T>(&response) {
        Ok(data) => Ok(data),
        Err(e) => {
            warn!(
                "Failed to get LLM response: {}. Falling back to default. Raw LLM output: {}",
                e, &response
            );
            Ok(T::default())
        }
    }
}

async fn get_ocr(llm: &LlamaClient, has_text: bool, img_path: &Path) -> Result<Option<String>> {
    if has_text {
        let ocr_result = llm.chat(prompts::OCR).images(&[img_path]).call().await;
        if ocr_result.is_err() {
            warn!("Couldn't OCR image: {}", img_path.display());
            Ok(None)
        } else {
            Ok(Some(ocr_result?))
        }
    } else {
        Ok(None)
    }
}

async fn get_detailed_analysis(
    llm: &LlamaClient,
    file: &Path,
    flags: &ClassifyFlags,
) -> Result<Option<ClassifyDetails>> {
    let mut props = json!({});
    let mut required = vec![];

    if flags.contains_animals || flags.contains_pets {
        props["animal_name"] = schemas::prop_string(
            "Best guess as to what animal is shown, give \
        a singular term naming the animal. Examples: [lion, giraffe, goose, fox, dog, ant]",
        );
        required.push("animal_name");
    }

    if flags.contains_vehicle {
        props["vehicle_type"] = json!({ "type": "string", "enum": schemas::VEHICLE_ENUM });
        required.push("vehicle_type");
    }

    if flags.famous_landmarks {
        props["landmark_name"] = schemas::prop_string("Name of the famous place or landmark");
        required.push("landmark_name");
    }

    if flags.contains_people {
        props["people_count"] = json!({ "type": "integer" });
        props["people_mood"] = json!({ "type": "string", "enum": schemas::MOOD_ENUM });
        props["photo_type"] = json!({ "type": "string", "enum": schemas::PHOTO_TYPE_ENUM });
        required.extend(["people_count", "people_mood", "photo_type"]);
    }

    if flags.depicts_food {
        props["food_name"] = schemas::prop_string("What food is shown in this photo?");
        required.push("food_name");
    }

    if flags.depicts_drink {
        props["drink_name"] = schemas::prop_string(
            "The main type of drink shown, examples: [coffee, tea, beer, wine, cola, water]",
        );
        required.push("drink_name");
    }

    if flags.depicts_event {
        props["event_type"] = json!({ "type": "string", "enum": schemas::EVENT_ENUM });
        required.push("event_type");
    }

    if flags.depicts_physical_activity {
        props["activity_name"] = schemas::prop_string(
            "The main physical action, activity, or sport being performed, examples: \
            [football, running, rowing, swimming]",
        );
        required.push("activity_name");
    }

    if flags.contains_document {
        props["document_type"] = json!({ "type": "string", "enum": schemas::DOCUMENT_ENUM });
        required.push("document_type");
    }

    if required.is_empty() {
        return Ok(None);
    }

    let dynamic_schema = json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false
    });

    request_structured_data(llm, file, prompts::DETAILS, dynamic_schema).await
}

mod prompts {
    pub const BASIC_INFO: &str = r"
Analyze this image and fill the schema fields.
caption:
    You are an image captioning assistant. Describe the main
    content of this image in a short, factual caption suitable for search. Include the main
    objects, people, animals, and the setting or scene, but do not add opinions or creative
    interpretations. Make it concise and clear, suitable for full-text search. Answer in one
    to two paragraphs.

subject:
    Concisely name the single main subject of the photo. Answer in one word.

setting:
    Identify the main scene or environment of this photo. Focus only on the type of
    place or setting (e.g., kitchen, classroom, street, park, courtyard, office). Ignore
    people, animals, objects, and activities. Answer with a word or short phrase suitable for
    categorizing or grouping photos.

search_term:
    What would a user type when looking for this image? Answer this in less than 6 words,
    preferrably less. Ideally the search_term is specific enough that this image would
    be first or near first in the search results, as long as you keep the hypothethical
    query short enough.
    Examples:
        * Hatchback in the mud
        * Woman eating pancackes
        * Towel on a sunny beach
";

    pub const CATEGORIZATION: &str = "You are an image categorization bot, analyze this image \
    and identify which categories it belongs to.";

    pub const DETAILS: &str = "You are an image classification bot. Analyze the provided photo to \
    fill the requested schema fields. Keep your answers as concise as possible, don't explain \
    your choices. Your answers will be used to organize and group photos.";

    pub const OCR: &str = "You are an OCR bot, transcribe the legible text in this image exactly \
    as is. Only OCR the pieces of text that are fully in view and clearly readable.";
}

mod schemas {
    use serde_json::{Value, json};

    pub fn prop_string(desc: &str) -> Value {
        json!({ "type": "string", "description": desc })
    }

    pub fn basic_info() -> Value {
        json!({
            "type": "object",
            "properties": {
                "caption": { "type": "string" },
                "main_subject": { "type": "string" },
                "setting": { "type": "string" },
                "search_term": { "type": "string" },
            },
            "required": ["setting", "main_subject", "caption"]
        })
    }

    pub fn categories() -> Value {
        json!({
            "type": "object",
            "properties": {
            "contains_pets": { "type": "boolean", "description": "Is there a prominent pet shown \
            in the photo? examples: [cat, dog, bird, etc.]" },

            "contains_animals": { "type": "boolean", "description": "Is an animal prominently \
            shown in the photo? Examples: [lion, dog, deer, ant, spider]" },

            "contains_vehicle": { "type": "boolean", "description": "Is a vehicle shown \
            prominently in the photo? Examples: [car, boat, bicycle, scooter]" },

            "famous_landmarks": { "type": "boolean", "description": "Does this photo clearly \
            show a *well-known, globally recognized landmark*? Examples: Eiffel Tower, Statue of \
            Liberty, Colosseum, Golden Gate Bridge, Parthenon. Do NOT mark typical buildings, \
            generic churches, or streetscapes as landmarks. Do NOT mark generic recognizable \
            natural features as landmarks. Mark as true only if a viewer could confidently name \
            the landmark without guessing." },

            "contains_people": { "type": "boolean", "description": "Does this photo contain at \
            least one person? Making sure the person is recognizable enough in the photo for you \
            to count the person(s) and detect their mood"},

            "is_indoor": { "type": "boolean" },

            "depicts_food": { "type": "boolean", "description": "Is this a photo food? Example: \
            [spaghetti, pizza, cheeseburger, lasagna]" },

            "depicts_drink": { "type": "boolean", "description": "Is this a photo a drink? Example: \
            [beer, coffee, cola, tea]" },

            "depicts_event": { "type": "boolean", "description": "Is this a specific event (e.g., \
            birthday party, wedding, concert, holiday)?" },

            "contains_document": { "type": "boolean", "description": "Is this a photo of one of the \
            following: passport, id_card, driver_license, certificate, receipt, invoice, bill, \
            bank_statement, tax_document, payslip, payment_card, ticket, boarding_pass, \
            reservation, itinerary, contract, license, warranty, manual, notes, exam_paper, \
            assignment, diploma, presentation_slide, book, magazine, newspaper, article, menu, \
            recipe, price_card, product_label, medical, letter, brochure, screenshot, diagram, \
            handwritten, sketch, generic_document." },

            "depicts_physical_activity": { "type": "boolean", "description": "Is a clear, intentional physical \
            action being performed in this photo (e.g., walking, cooking, exercising), not \
            including passive states such as sitting, standing, resting, posing, or watching?" },

            "contains_legible_text": { "type": "boolean", "description": "Is there legible text visible in \
            this photo?" },

            "depicts_landscape": { "type": "boolean", "description": "Is this a landscape featuring \
            natural scenery such as mountains, dunes, forests, lakes, etc.?" },

            "depicts_cityscape": { "type": "boolean" },
            },
            "required": [
                "contains_pets", "contains_animals", "contains_vehicle", "famous_landmarks",
                "contains_people", "is_indoor", "depicts_food", "depicts_drink", "depicts_event",
                "contains_document", "depicts_physical_activity", "contains_legible_text",
                "depicts_landscape", "depicts_cityscape"
            ]
        })
    }

    pub const VEHICLE_ENUM: &[&str] = &[
        "car",
        "motorcycle",
        "bicycle",
        "scooter",
        "moped",
        "truck",
        "van",
        "bus",
        "tram",
        "train",
        "subway",
        "trolleybus",
        "rickshaw",
        "electric_scooter",
        "segway",
        "boat",
        "ferry",
        "yacht",
        "sailboat",
        "canoe",
        "kayak",
        "jet_ski",
        "rowboat",
        "speedboat",
        "ship",
        "airplane",
        "helicopter",
        "glider",
        "hot_air_balloon",
        "drone",
        "tramcar",
        "cable_car",
        "funicular",
        "monorail",
        "submarine",
        "horse_cart",
        "animal_cart",
        "camper",
        "other_vehicle",
    ];

    pub const MOOD_ENUM: &[&str] = &[
        "happy",
        "content",
        "relaxed",
        "calm",
        "excited",
        "playful",
        "energetic",
        "focused",
        "thoughtful",
        "neutral",
        "serious",
        "tired",
        "bored",
        "sad",
        "melancholic",
        "anxious",
        "stressed",
        "frustrated",
        "angry",
        "confident",
    ];

    pub const PHOTO_TYPE_ENUM: &[&str] = &[
        "selfie",
        "group photo",
        "crowd",
        "portrait",
        "action",
        "candid",
        "people_not_main_subject",
        "other",
    ];

    pub const EVENT_ENUM: &[&str] = &[
        "holiday",
        "wedding",
        "birthday_party",
        "new_years_eve",
        "christmas",
        "easter",
        "anniversary",
        "graduation",
        "party",
        "baby_shower",
        "funeral",
        "concert",
        "festival",
        "sports_event",
        "parade",
        "theater_performance",
        "exhibition",
        "conference",
        "workshop",
        "trade_show",
        "family_gathering",
        "reunion",
        "picnic",
        "barbecue",
        "camping_trip",
        "protest",
        "political_rally",
        "charity_event",
        "other_event",
    ];

    pub const DOCUMENT_ENUM: &[&str] = &[
        "passport",
        "id_card",
        "driver_license",
        "certificate",
        "receipt",
        "invoice",
        "bill",
        "bank_statement",
        "tax_document",
        "payslip",
        "payment_card",
        "ticket",
        "boarding_pass",
        "reservation",
        "itinerary",
        "contract",
        "license",
        "warranty",
        "manual",
        "notes",
        "exam_paper",
        "assignment",
        "diploma",
        "presentation_slide",
        "book",
        "magazine",
        "newspaper",
        "article",
        "menu",
        "recipe",
        "price_card",
        "product_label",
        "medical",
        "letter",
        "brochure",
        "screenshot",
        "diagram",
        "handwritten",
        "sketch",
        "generic_document",
    ];
}

fn map_to_final_result(
    basic: BasicClassifyCaption,
    categories: &ClassifyFlags,
    details: Option<ClassifyDetails>,
    ocr: Option<String>,
) -> MLLlmClassification {
    let det = details.unwrap_or_default();

    MLLlmClassification {
        caption: basic.caption,
        main_subject: basic.main_subject.to_lowercase(),
        setting: basic.setting.to_lowercase(),
        search_term: basic.search_term.to_lowercase(),

        contains_pets: categories.contains_pets,
        contains_vehicle: categories.contains_vehicle,
        contains_landmarks: categories.famous_landmarks,
        contains_animals: categories.contains_animals,
        is_indoor: categories.is_indoor,
        is_food: categories.depicts_food,
        is_drink: categories.depicts_drink,
        is_event: categories.depicts_event,
        is_document: categories.contains_document,
        is_landscape: categories.depicts_landscape,
        is_cityscape: categories.depicts_cityscape,
        is_activity: categories.depicts_physical_activity,
        contains_text: categories.contains_legible_text,

        ocr_text: ocr,
        document_type: det.document_type,
        animal_type: det.animal_name.map(|a| a.to_lowercase()),
        food_name: det.food_name.map(|a| a.to_lowercase()),
        drink_name: det.drink_name.map(|a| a.to_lowercase()),
        vehicle_type: det.vehicle_type,
        event_type: det.event_type,
        landmark_name: det.landmark_name,
        activity_name: det.activity_name.map(|a| a.to_lowercase()),
        people_mood: det.people_mood,
        photo_type: det.photo_type,
        people_count: det.people_count,

        contains_people: categories.contains_people && det.people_count.unwrap_or(0) > 0,
    }
}
