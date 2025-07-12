use std::io;
use std::net::TcpListener;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::configuration::{DatabaseSettings, Settings};
use crate::routes::{get_confirm, get_health, post_subscriptions};
use crate::{EmailClient, telemetry::tracing_layer};

#[derive(Debug)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: String,
}

#[derive(Debug)]
pub struct Application {
    listener: TcpListener,
    port: u16,
    router: Router,
}

impl Application {
    pub fn build(configuration: Settings) -> io::Result<Self> {
        let db_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let shared_state = Arc::new(AppState {
            db_pool,
            email_client,
            base_url: configuration.application.base_url,
        });

        let mut router = Router::new()
            .route("/health", get(get_health))
            .route("/subscriptions", post(post_subscriptions))
            .route("/subscriptions/confirm", get(get_confirm))
            .with_state(shared_state);

        router = tracing_layer(router);

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr()?.port();

        Ok(Self {
            listener,
            port,
            router,
        })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn router(self) -> Router {
        self.router
    }

    pub async fn run_until_stopped(self) -> io::Result<()> {
        self.listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(self.listener)?;
        axum::serve(listener, self.router).await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}
