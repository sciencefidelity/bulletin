use axum::http::StatusCode;

pub async fn get_health() -> StatusCode {
    StatusCode::OK
}
