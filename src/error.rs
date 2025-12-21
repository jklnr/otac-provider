use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OtacError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Token creation failed: {0}")]
    TokenCreation(String),

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token not found or already consumed")]
    TokenNotFound,

    #[error("Token expired")]
    TokenExpired,

    #[error("Bad request: {0}")]
    BadRequest(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

impl IntoResponse for OtacError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            OtacError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
            OtacError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR"),
            OtacError::TokenCreation(_) => (StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_CREATION_ERROR"),
            OtacError::InvalidToken(_) => (StatusCode::BAD_REQUEST, "INVALID_TOKEN"),
            OtacError::TokenNotFound => (StatusCode::GONE, "TOKEN_NOT_FOUND"),
            OtacError::TokenExpired => (StatusCode::GONE, "TOKEN_EXPIRED"),
            OtacError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

