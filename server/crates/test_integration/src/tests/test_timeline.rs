use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::eyre::{Result, bail};
use common_types::pb::api::{TimelineItemsResponse, TimelineRatiosResponse};
use prost::Message;
use reqwest::StatusCode;
use std::collections::HashSet;
use std::io::Cursor;

/// Tests the `GET /timeline/ids` endpoint.
pub async fn test_get_timeline_ids(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let client = &context.http_client;
    let url = format!("{}/timeline/ids", context.settings.api.public_url);

    // ACT
    let response = client.get(&url).send().await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    let response_ids: HashSet<String> = response.json().await?;
    let db_ids: HashSet<String> = sqlx::query_scalar!("SELECT id FROM media_item")
        .fetch_all(&context.pool)
        .await?
        .into_iter()
        .collect();

    assert_eq!(db_ids, response_ids);

    Ok(())
}

/// Tests the `GET /timeline/ratios` endpoint.
pub async fn test_get_timeline_ratios(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let client = &context.http_client;
    let url = format!("{}/timeline/ratios", context.settings.api.public_url);

    // ACT
    let response = client.get(&url).send().await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.bytes().await?;
    let timeline = TimelineRatiosResponse::decode(Cursor::new(bytes))?;

    if let Some(month) = timeline.months.first() {
        assert!(!month.month_id.is_empty(), "Month ID should not be empty");
        assert!(month.count > 0, "Month item count should be positive");
    }

    Ok(())
}

/// Tests the `GET /timeline/by-month` endpoint.
pub async fn test_get_photos_by_month(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let client = &context.http_client;

    let ratios_url = format!("{}/timeline/ratios", context.settings.api.public_url);
    let ratio_res = client.get(&ratios_url).send().await?;
    let ratio_bytes = ratio_res.bytes().await?;
    let timeline = TimelineRatiosResponse::decode(Cursor::new(ratio_bytes))?;

    let Some(first_month) = timeline.months.first() else {
        bail!("No first month returned from timeline/ratios endpoint.");
    };
    let target_month = &first_month.month_id;
    let url = format!("{}/timeline/by-month", context.settings.api.public_url);

    // ACT
    let response = client
        .get(&url)
        .query(&[("months", target_month)])
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.bytes().await?;
    let by_month = TimelineItemsResponse::decode(Cursor::new(bytes))?;

    assert!(
        !by_month.months.is_empty(),
        "Should return data for the requested month"
    );

    let returned_group = &by_month.months[0];
    assert!(
        !returned_group.items.is_empty(),
        "Month should contain media items"
    );

    let item = &returned_group.items[0];
    assert!(!item.id.is_empty(), "Media item ID should be present");

    Ok(())
}