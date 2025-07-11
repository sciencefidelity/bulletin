use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use axum::routing::{get, post};
use axum::{Form, Router, http::StatusCode};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};

use crate::routes::{get_health, post_subscriptions};

pub fn run(address: &str, db_pool: PgPool) -> Result<(TcpListener, Router), std::io::Error> {
    let connection = Arc::new(db_pool);

    let router = Router::new()
        .route("/health", get(get_health))
        .route("/subscriptions", post(post_subscriptions))
        .with_state(connection);

    let listener = TcpListener::bind(address)?;

    Ok((listener, router))
}
