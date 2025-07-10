use anyhow::Result;
use reqwest::{StatusCode, header::CONTENT_TYPE};

#[tokio::test]
async fn health_check_works() -> Result<()> {
    let app_address = spawn_app()?;
    let client = reqwest::Client::new();

    let response = client.get(format!("{app_address}/health")).send().await?;

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    let app_address = spawn_app()?;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{app_address}/subscriptions"))
        .header(CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())
        .body(body)
        .send()
        .await?;

    assert_eq!(StatusCode::OK, response.status());

    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() -> Result<()> {
    let app_address = spawn_app()?;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{app_address}/subscriptions"))
            .header(CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())
            .body(invalid_body)
            .send()
            .await?;

        assert_eq!(
            StatusCode::UNPROCESSABLE_ENTITY,
            response.status(),
            "The API did not fail with 400 Bad Request when the payload was {error_message}.",
        );
    }

    Ok(())
}

fn spawn_app() -> Result<String> {
    let (listener, router) = bulletin::run("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = axum_server::from_tcp(listener).serve(router.into_make_service());
    tokio::spawn(server);
    Ok(format!("http://127.0.0.1:{port}"))
}
