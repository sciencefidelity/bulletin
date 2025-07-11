use std::sync::LazyLock;

use anyhow::Result;
use bulletin::configuration::{self, DatabaseSettings};
use bulletin::telemetry::{Formatter, get_subscriber, init_subscriber};
use reqwest::{StatusCode, header::CONTENT_TYPE};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_log_level = "info".to_owned();
    let subscriber_name = "test".to_owned();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name,
            default_log_level,
            &Formatter::Bunyan,
            std::io::stdout,
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name,
            default_log_level,
            &Formatter::Bunyan,
            std::io::sink,
        );
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> Result<TestApp> {
    LazyLock::force(&TRACING);

    let mut configuration = configuration::get()?;
    configuration.database.name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&configuration.database).await?;

    let (listener, router) = bulletin::run("127.0.0.1:0", db_pool.clone())?;
    let port = listener.local_addr()?.port();
    let address = format!("http://127.0.0.1:{port}");

    let server = axum_server::from_tcp(listener).serve(router.into_make_service());
    tokio::spawn(server);
    Ok(TestApp { address, db_pool })
}

async fn configure_database(config: &DatabaseSettings) -> Result<PgPool> {
    let mut connection = PgConnection::connect_with(&config.without_db()).await?;
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.name).as_str())
        .await?;
    let connection_pool = PgPool::connect_with(config.with_db()).await?;
    sqlx::migrate!("./migrations").run(&connection_pool).await?;
    Ok(connection_pool)
}
#[tokio::test]
async fn health_check_works() -> Result<()> {
    let app = spawn_app().await?;
    let client = reqwest::Client::new();

    let response = client.get(format!("{}/health", app.address)).send().await?;

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    let app = spawn_app().await?;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app.address))
        .header(CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())
        .body(body)
        .send()
        .await?;

    assert_eq!(StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await?;

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() -> Result<()> {
    let app = spawn_app().await?;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", app.address))
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

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_invalid() -> Result<()> {
    let app = spawn_app().await?;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        assert_eq!(
            StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not return a 400 OK when the payload was {description}.",
        );
    }

    Ok(())
}
