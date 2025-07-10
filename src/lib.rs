#![allow(unused, clippy::missing_errors_doc, clippy::missing_panics_doc)]
use std::net::TcpListener;

use axum::{
    Form, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn get_health() -> StatusCode {
    StatusCode::OK
}

async fn post_subscriptions(Form(_form): Form<FormData>) -> StatusCode {
    StatusCode::OK
}

pub fn run(address: &str) -> Result<(TcpListener, Router), std::io::Error> {
    let router = Router::new()
        .route("/health", get(get_health))
        .route("/subscriptions", post(post_subscriptions));

    let listener = TcpListener::bind(address)?;

    Ok((listener, router))
}
