use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::eyre::Result;
use common_types::pb::api::SearchResponse;
use prost::Message;
use reqwest::StatusCode;
use std::io::Cursor;

pub async fn test_search_filters(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let url = format!("{}/search", context.settings.api.public_url);

    // --- TEST 1: Basic Search ---
    let res = client
        .get(&url)
        .query(&[("query", "photo")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.bytes().await?;
    let search_res = SearchResponse::decode(Cursor::new(bytes))?;
    assert!(
        !search_res.items.is_empty(),
        "Should return some items for basic search"
    );

    // --- TEST 2: Filter by Media Type (Photo) ---
    let res = client
        .get(&url)
        .query(&[("query", "photo"), ("mediaType", "photo")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let search_res = SearchResponse::decode(Cursor::new(res.bytes().await?))?;
    for item in &search_res.items {
        assert!(
            !item.is_video,
            "Item should not be a video when mediaType=photo"
        );
    }

    // --- TEST 3: Filter by Media Type (Video) ---
    let res = client
        .get(&url)
        .query(&[("query", "video"), ("mediaType", "video")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let search_res = SearchResponse::decode(Cursor::new(res.bytes().await?))?;
    for item in &search_res.items {
        assert!(item.is_video, "Item should be a video when mediaType=video");
    }

    // --- TEST 4: Sort by Date ---
    let res = client
        .get(&url)
        .query(&[("query", "photo"), ("sortBy", "date")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let search_res = SearchResponse::decode(Cursor::new(res.bytes().await?))?;
    assert!(!search_res.items.is_empty());

    // --- TEST 5: Negative Query ---
    let res = client
        .get(&url)
        .query(&[("query", "dog"), ("negativeQuery", "cat")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);

    // --- TEST 6: Datetime Filter ---
    let res = client
        .get(&url)
        .query(&[
            ("query", "photo"),
            ("startDate", "2020-01-01T00:00:00Z"),
            ("endDate", "2030-01-01T00:00:00Z"),
        ])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);

    // --- TEST 7: Country Code Filter ---
    let res = client
        .get(&url)
        .query(&[("query", "photo"), ("countryCode", "NL")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);

    Ok(())
}
