use crate::api::search::interfaces::{SearchMediaConfig, SearchParams};
use app_state::SearchSettings;

#[must_use]
pub fn to_search_config(
    search_settings: &SearchSettings,
    search_params: SearchParams,
) -> SearchMediaConfig {
    SearchMediaConfig {
        embedder_model_id: search_settings.embedder_model_id.clone(),
        semantic_score_threshold: search_settings.semantic_score_threshold,
        text_weight: search_settings.text_weight,
        semantic_weight: search_settings.semantic_weight,
        limit: search_params.limit,
        offset: search_params.offset,
        start_date: search_params.start_date,
        end_date: search_params.end_date,
        media_type: search_params.media_type,
        sort_by: search_params.sort_by,
        negative_query: search_params.negative_query,
        country_codes: search_params
            .country_codes
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        person_ids: search_params
            .person_ids
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        all_faces_required: search_params.all_faces_required.unwrap_or_default(),
    }
}
