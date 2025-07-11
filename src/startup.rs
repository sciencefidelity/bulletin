use std::net::TcpListener;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;

use crate::routes::{get_health, post_subscriptions};
use crate::telemetry::tracing_layer;

pub fn run(address: &str, db_pool: PgPool) -> Result<(TcpListener, Router), std::io::Error> {
    let connection = Arc::new(db_pool);

    let mut router = Router::new()
        .route("/health", get(get_health))
        .route("/subscriptions", post(post_subscriptions))
        .with_state(connection);

    router = tracing_layer(router);

    let listener = TcpListener::bind(address)?;

    Ok((listener, router))
}
