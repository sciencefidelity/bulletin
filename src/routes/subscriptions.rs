use std::sync::Arc;

use axum::{Form, extract::State, http::StatusCode};
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::Deserialize;
use sqlx::types::chrono::Utc;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::EmailClient;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::startup::AppState;

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
    State(state): State<Arc<AppState>>,
    Form(form): Form<FormData>,
) -> StatusCode {
    let Ok(new_subscriber) = form.try_into() else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(mut transaction) = state.db_pool.begin().await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    let Ok(subscriber_id) = insert_subscriber(&mut transaction, &new_subscriber).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if transaction.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

#[tracing::instrument(name = "saving new subscriber details in the database", skip_all)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("failed to execute query: {e:?}");
        e
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "store subscription token in the database", skip_all)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "send a confirmation email to a new subscriber", skip_all)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription"
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}
