use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::error::OtacError;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateRequest {
    /// The subject/identifier to store in the token
    /// This will be returned when the token is consumed
    pub subject: String,
}

#[derive(Serialize)]
pub struct CreateResponse {
    /// The signed JWT token
    pub token: String,
}

/// POST /create
/// 
/// Create a new one-time access code.
/// 
/// Request body:
/// ```json
/// { "subject": "user@example.com" }
/// ```
/// 
/// Response:
/// ```json
/// { "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS0xIn0..." }
/// ```
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<CreateResponse>, OtacError> {
    let subject = body.subject.trim();
    
    if subject.is_empty() {
        return Err(OtacError::BadRequest("subject is required".into()));
    }

    let token = state.otac_service.create(subject).await?;

    Ok(Json(CreateResponse { token }))
}

#[derive(Deserialize)]
pub struct ConsumeRequest {
    /// The token to consume
    pub token: String,
}

#[derive(Serialize)]
pub struct ConsumeResponse {
    /// The subject that was stored in the token
    pub subject: String,
}

/// POST /consume
/// 
/// Consume a one-time access code.
/// 
/// Request body:
/// ```json
/// { "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS0xIn0..." }
/// ```
/// 
/// Response (success):
/// ```json
/// { "subject": "user@example.com" }
/// ```
/// 
/// Response (already consumed or invalid):
/// ```json
/// { "error": "Token not found or already consumed", "code": "TOKEN_NOT_FOUND" }
/// ```
pub async fn consume(
    State(state): State<AppState>,
    Json(body): Json<ConsumeRequest>,
) -> Result<Json<ConsumeResponse>, OtacError> {
    let token = body.token.trim();
    
    if token.is_empty() {
        return Err(OtacError::BadRequest("token is required".into()));
    }

    let subject = state.otac_service.consume(token).await?;

    Ok(Json(ConsumeResponse { subject }))
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// GET /health
/// 
/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

