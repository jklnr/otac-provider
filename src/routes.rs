use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::OtacError;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateRequest {
    /// Arbitrary JSON payload to store in the token
    /// This will be returned exactly as-is when the token is consumed
    pub payload: Value,
}

#[derive(Serialize)]
pub struct CreateResponse {
    /// The signed JWT token
    pub token: String,
}

/// POST /create
/// 
/// Create a new one-time access code with arbitrary payload.
/// 
/// Request body:
/// ```json
/// { "payload": { "username": "jay", "action": "reset" } }
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
    if body.payload.is_null() {
        return Err(OtacError::BadRequest("payload is required".into()));
    }

    let token = state.otac_service.create(&body.payload).await?;

    Ok(Json(CreateResponse { token }))
}

#[derive(Deserialize)]
pub struct ConsumeRequest {
    /// The token to consume
    pub token: String,
}

#[derive(Serialize)]
pub struct ConsumeResponse {
    /// The payload that was stored in the token
    pub payload: Value,
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
/// { "payload": { "username": "jay", "action": "reset" } }
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

    let payload = state.otac_service.consume(token).await?;

    Ok(Json(ConsumeResponse { payload }))
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
