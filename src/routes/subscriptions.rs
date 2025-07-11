use std::sync::Arc;

use axum::{Form, extract::State, http::StatusCode};
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::types::chrono::Utc;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "adding a new subscriber",
    skip_all,
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn post_subscriptions(
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<FormData>,
) -> StatusCode {
    let Ok(new_subscriber) = form.try_into() else {
        return StatusCode::BAD_REQUEST;
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "saving new subscriber details in the database", skip_all)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}
