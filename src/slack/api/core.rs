use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::slack::client;

#[derive(Debug)]
pub enum ApiError {
    InternalServerError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::InternalServerError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl From<client::Error> for ApiError {
    fn from(err: client::Error) -> Self {
        ApiError::InternalServerError(format!("client error: {}", err.message))
    }
}

