use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

pub enum ApiError {
    NotFound,
    Forbidden,
    BadRequest(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Internal(e) => {
                tracing::error!("internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<link_shortener_store::StoreError> for ApiError {
    fn from(e: link_shortener_store::StoreError) -> Self {
        match &e {
            link_shortener_store::StoreError::LinkNotFound(_) => ApiError::NotFound,
            link_shortener_store::StoreError::SlugConflict(slug) => {
                ApiError::Conflict(format!("slug already exists: {slug}"))
            }
            _ => ApiError::Internal(e.into()),
        }
    }
}
