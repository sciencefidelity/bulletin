use std::io;
use std::net::TcpListener;
use std::sync::Arc;

use axum::Router;
// use axum::http::Request;
use axum::routing::{get, post};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
// use tower::ServiceBuilder;
// use tower_http::ServiceBuilderExt;
// use tower_http::request_id::{MakeRequestId, RequestId};
// use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
// use uuid::Uuid;

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
            configuration.email_client.api_token,
            timeout,
        );

        let shared_state = Arc::new(AppState {
            db_pool,
            email_client,
            base_url: configuration.application.base_url,
        });

        // let svc = ServiceBuilder::new()
        //     .set_x_request_id(MakeRequestUuid)
        //     .layer(
        //         TraceLayer::new_for_http()
        //             .make_span_with(DefaultMakeSpan::new().include_headers(true))
        //             .on_response(DefaultOnResponse::new().include_headers(true)),
        //     )
        //     .propagate_x_request_id();

        let mut router = Router::new()
            .route("/health", get(get_health))
            .route("/subscriptions", post(post_subscriptions))
            .route("/subscriptions/confirm", get(get_confirm))
            // .layer(svc)
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

// #[derive(Clone)]
// struct MakeRequestUuid;
//
// impl MakeRequestId for MakeRequestUuid {
//     fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
//         let request_id = Uuid::new_v4().to_string();
//         Some(RequestId::new(
//             request_id.parse().expect("failed to parse request uuid"),
//         ))
//     }
// }
