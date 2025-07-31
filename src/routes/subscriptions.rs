use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{Form, extract::State};
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::Deserialize;
use sqlx::types::chrono::Utc;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::EmailClient;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::error::{HttpError, Result};
use crate::startup::AppState;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "POST - new subscription",
    skip_all,
    fields(email = %form.email)
)]
pub async fn post_subscriptions(
    State(state): State<Arc<AppState>>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse> {
    let new_subscriber = form.try_into().map_err(|e| HttpError::ValidationError(e))?;

    let mut transaction = state
        .db_pool
        .begin()
        .await
        .map_err(|e| HttpError::DatabaseError(e))?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(|e| HttpError::DatabaseError(e))?;

    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .map_err(|e| HttpError::DatabaseError(e))?;

    transaction
        .commit()
        .await
        .map_err(|e| HttpError::DatabaseError(e))?;

    send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .map_err(|_| HttpError::UnexpectedError)?;

    Ok(())
}

#[tracing::instrument(
    name = "writing new subscriber to the database",
    skip_all,
    fields(email = %new_subscriber.email.as_ref())
)]
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
        tracing::error!("execute insert_subscriber: {e:?}");
        e
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "writing subscription token to the database", skip_all)]
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
        tracing::error!("execute store_token: {e:?}");
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "sending a confirmation email", skip_all)]
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
