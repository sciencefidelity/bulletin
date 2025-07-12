use anyhow::Result;

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() -> Result<()> {
    let app = spawn_app().await?;
    let client = reqwest::Client::new();

    let response = client.get(format!("{}/health", app.address)).send().await?;

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    Ok(())
}
