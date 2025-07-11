use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use axum::{Form, extract::State, http::StatusCode};
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::{Executor, PgConnection, types::chrono::Utc};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn post_subscriptions(
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<FormData>,
) -> StatusCode {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("failed to execute query: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
