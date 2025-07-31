use std::sync::LazyLock;

use anyhow::Result;
use bulletin::Application;
use bulletin::configuration::{self, DatabaseSettings};
use bulletin::startup::get_connection_pool;
use bulletin::telemetry::{Formatter, get_subscriber, init_subscriber};
use reqwest::header::CONTENT_TYPE;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_log_level = "info".to_owned();
    let subscriber_name = "test".to_owned();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name,
            default_log_level,
            &Formatter::Log,
            std::io::stdout,
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name,
            default_log_level,
            &Formatter::Log,
            std::io::sink,
        );
        init_subscriber(subscriber);
    }
});

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: &str) -> Result<reqwest::Response> {
        Ok(reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header(
                CONTENT_TYPE,
                mime::APPLICATION_WWW_FORM_URLENCODED.to_string(),
            )
            .body(body.to_owned())
            .send()
            .await?)
    }

    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> Result<ConfirmationLinks> {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body)?;

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        Ok(ConfirmationLinks { html, plain_text })
    }
}

pub async fn spawn_app() -> Result<TestApp> {
    LazyLock::force(&TRACING);
    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = configuration::get()?;
        c.database.name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await?;
    let db_pool = get_connection_pool(&configuration.database);

    let application = Application::build(configuration)?;
    let port = application.port();
    let router = application.router();

    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))?;
    let port = listener.local_addr()?.port();
    let address = format!("http://127.0.0.1:{port}");

    let server = axum_server::from_tcp(listener).serve(router.into_make_service());
    tokio::spawn(server);
    Ok(TestApp {
        address,
        db_pool,
        email_server,
        port,
    })
}

async fn configure_database(config: &DatabaseSettings) -> Result<()> {
    let mut connection = PgConnection::connect_with(&config.without_db()).await?;
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.name).as_str())
        .await?;
    let connection_pool = PgPool::connect_with(config.with_db()).await?;
    sqlx::migrate!("./migrations").run(&connection_pool).await?;

    Ok(())
}
