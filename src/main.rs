mod config;
mod error;
mod otac;
mod routes;

use axum::{routing::post, Router};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::otac::OtacService;

#[derive(Clone)]
pub struct AppState {
    pub otac_service: Arc<OtacService>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "otac-provider=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!(
        storage = %config.storage_url_display(),
        ttl_seconds = config.token_ttl_seconds,
        "Starting otac-provider"
    );

    // Initialize the OTAC service
    let otac_service = OtacService::new(config).await?;
    let state = AppState {
        otac_service: Arc::new(otac_service),
    };

    // Build the router
    let app = Router::new()
        .route("/create", post(routes::create))
        .route("/consume", post(routes::consume))
        .route("/health", axum::routing::get(routes::health))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start the server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!(%addr, "Listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

