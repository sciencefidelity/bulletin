use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{HttpError, Result};
use crate::startup::AppState;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "confirm a pending subscriber", skip_all)]
pub async fn get_confirm(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Parameters>,
) -> Result<impl IntoResponse> {
    let id = get_subscriber_id_from_token(&state.db_pool, &params.subscription_token)
        .await
        .map_err(HttpError::DatabaseError)?;

    match id {
        None => {
            return Err(HttpError::AuthorizationError(
                "no matching subscriber id for provided token".into(),
            ))?;
        }
        Some(subscriber_id) => {
            confirm_subscriber(&state.db_pool, subscriber_id)
                .await
                .map_err(HttpError::DatabaseError)?;

            Ok(())
        }
    }
}

#[tracing::instrument(name = "get subscriber id from token", skip_all)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token =
$1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query: {e:?}");
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "mark subscriber as confirmed", skip_all)]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"update subscriptions set status = 'confirmed' where id =
$1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query: {e:?}");
        e
    })?;

    Ok(())
}
