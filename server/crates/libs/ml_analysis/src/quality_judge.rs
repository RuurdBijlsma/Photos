use color_eyre::eyre::Result;
use common_types::ml_analysis::MLLlmQualityJudgement;
use language_model::LlamaClient;
use serde_json::json;
use std::path::Path;

pub async fn get_quality_judgement(
    client: &LlamaClient,
    file: &Path,
) -> Result<Option<MLLlmQualityJudgement>> {
    let judgement_schema: serde_json::Value = json!({
        "type": "object",
        "properties": {
            "exposure": { "type": "integer", "minimum": 1, "maximum": 10 },
            "contrast": { "type": "integer", "minimum": 1, "maximum": 10 },
            "sharpness": { "type": "integer", "minimum": 1, "maximum": 10 },
            "color_accuracy": { "type": "integer", "minimum": 1, "maximum": 10 },
            "composition": { "type": "integer", "minimum": 1, "maximum": 10 },
            "subject_clarity": { "type": "integer", "minimum": 1, "maximum": 10 },
            "visual_impact": { "type": "integer", "minimum": 1, "maximum": 10 },
            "creativity": { "type": "integer", "minimum": 1, "maximum": 10 },
            "color_harmony": { "type": "integer", "minimum": 1, "maximum": 10 },
            "storytelling": { "type": "integer", "minimum": 1, "maximum": 10 },
            "style_suitability": { "type": "integer", "minimum": 1, "maximum": 10 }
        },
        "required": [
            "exposure", "contrast", "sharpness", "color_accuracy",
            "composition", "subject_clarity", "visual_impact",
            "creativity", "color_harmony", "storytelling", "style_suitability"
        ],
        "additionalProperties": false
    });
    let prompt = r#"
Analyze this photo and evaluate it on the following criteria. For each category, provide a score from 1 to 10.
Use the following meanings for each score: 1 = very poor, 5 = meh, 6 = acceptable, 7 = good, 8 = great, 9 = BEAUTIFUL, 10 = IMPOSSIBLY GOOD.
Use a realistic distribution for scores. In most average photos, scores should often fall between 4 and 7.
Only award 9–10 for truly exceptional quality. Avoid giving perfect scores unless the photo is outstanding.

{
  "exposure": ?,        // How well-exposed the image is. 1 = extremely under/overexposed, 10 = perfectly balanced exposure.
  "contrast": ?,        // How well tones are separated. 1 = flat, muddy tones; 10 = clear, strong tonal separation.
  "sharpness": ?,       // Focus and clarity of main subjects. 1 = very blurry or motion blurred; 10 = perfectly sharp where intended.
  "color_accuracy": ?,  // How natural and accurate colors appear. 1 = unrealistic or off colors; 10 = colors perfectly natural and pleasing.
  "composition": ?,     // How well the photo is composed (rule of thirds, leading lines, framing, symmetry). 1 = poor composition; 10 = excellent composition.
  "subject_clarity": ?, // How clearly the main subject is presented. 1 = hard to tell what the subject is; 10 = main subject is clear and immediately recognizable.
  "visual_impact": ?,   // Emotional or visual effect of the photo. 1 = dull or unengaging; 10 = striking, compelling, evokes strong emotion.
  "creativity": ?,      // Originality, perspective, unusual angles, or lighting. 1 = generic, uninspired; 10 = very creative and original.
  "color_harmony": ?,   // How pleasing the color palette is. 1 = clashing or unappealing colors; 10 = harmonious, aesthetically pleasing colors.
  "storytelling": ?,    // Does the photo convey a story, mood, or context? 1 = no story or unclear context; 10 = strong narrative or context is immediately clear.
  "style_suitability": ? // Appropriateness for its genre (portrait, landscape, macro, street, etc.). 1 = unsuitable or mismatched; 10 = perfectly suited to its style/genre.
}
    "#;
    let response_text = client
        .chat(prompt)
        .images(&[file])
        .schema(judgement_schema.clone())
        .call()
        .await?;

    let deserialize_result = serde_json::from_str::<MLLlmQualityJudgement>(&response_text);
    Ok(deserialize_result.ok())
}
