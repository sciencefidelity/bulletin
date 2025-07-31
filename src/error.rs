use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub type Result<T, E = Report> = color_eyre::Result<T, E>;

pub struct Report(color_eyre::Report);

impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E> From<E> for Report
where
    E: Into<color_eyre::Report>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for Report {
    fn into_response(self) -> Response {
        let err = self.0;
        let err_string = format!("{err:?}");

        tracing::error!("{err_string}");

        if let Some(err) = err.downcast_ref::<HttpError>() {
            return err.response();
        }

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": {
                    "type": "SERVICE_ERROR",
                },
            })),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("validation error: {0}")]
    ValidationError(String),
    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("authorization error: {0}")]
    AuthorizationError(String),
    #[error("not found")]
    NotFound,
    #[error("confilct: {0}")]
    Conflict(String),
    #[error("unexpected error")]
    UnexpectedError,
}

impl HttpError {
    pub fn response(&self) -> Response {
        let (status, message) = match self {
            Self::ValidationError(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            Self::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SERVICE_ERROR"),
            Self::AuthorizationError(_) => (StatusCode::UNAUTHORIZED, "AUTHORIZATION_ERROR"),
            Self::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            Self::UnexpectedError => (StatusCode::INTERNAL_SERVER_ERROR, "SERVICE_ERROR"),
        };

        let client_body_error = json!({
            "error": {
                "type": message,
            }
        });

        (status, Json(client_body_error)).into_response()
    }
}
