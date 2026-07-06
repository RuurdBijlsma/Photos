#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use app_state::{database_url, load_app_settings};
use common_services::api::search::interfaces::{SearchMediaConfig, SearchMediaType, SearchSortBy};
use common_services::api::search::service::search_media;
use common_services::database::get_db_pool;
use common_services::database::user_store::UserStore;
use common_types::dev_constants::EMAIL;
use open_clip_inference::TextEmbedder;
use std::sync::Arc;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), true).await?;

    let user = UserStore::find_by_email(&pool, EMAIL)
        .await?
        .expect("no such user");
    let embedder = TextEmbedder::from_hf(&settings.ingest.analyzer.search.embedder_model_id)
        .build()
        .await?;

    let mut i = 0;
    let embedder = Arc::new(embedder);
    loop {
        let search_results = search_media(
            &user,
            &pool,
            embedder.clone(),
            Some("cat".to_owned()),
            SearchMediaConfig {
                embedder_model_id: settings.ingest.analyzer.search.embedder_model_id.clone(),
                text_weight: 0.5,
                semantic_weight: 0.5,
                semantic_score_threshold: 0.8,
                limit: Some(250),
                offset: Some(0),
                media_type: SearchMediaType::All,
                sort_by: SearchSortBy::Relevancy,
                country_codes: vec![],
                person_ids: vec![],
                all_faces_required: false,
                negative_query: None,
                start_date: None,
                end_date: None,
            },
        )
        .await?;
        println!(
            "{i} | results count: {:?} | {}",
            search_results.len(),
            if search_results.len() == 250 {
                "[✅ ]"
            } else {
                "[⛔ ]"
            }
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        i += 1;
        if i > 30 {
            break;
        }
    }
    // println!("Search took {:?}", now.elapsed());
    //
    // println!(
    //     "search result: {}",
    //     serde_json::to_string_pretty(&search_results)?
    // );

    Ok(())
}
