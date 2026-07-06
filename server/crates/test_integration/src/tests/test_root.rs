use crate::runner::context::test_context::TestContext;

pub async fn test_health_endpoint(context: &TestContext) -> color_eyre::Result<()> {
    // ARRANGE
    let url = format!("{}/health", &context.settings.api.public_url);

    // ACT
    let response = context.http_client.get(url).send().await?;
    let status = response.status();
    let body = response.text().await?;

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body, "OK");

    Ok(())
}
