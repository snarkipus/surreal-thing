use axum::body::Body;
use axum_macros::FromRef;
use once_cell::sync::Lazy;
use telemetry::{get_subscriber, init_subscriber};
use tower_http::trace::TraceLayer;
use tracing::info;

pub mod api;
pub mod db;
pub mod error;
pub mod telemetry;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router, Server};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::db::{Database, DatabaseSettings};

// region: -- conditional tracing for tests
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});
// endregion: -- conditional tracing for tests

#[derive(Debug, Clone, FromRef)]
pub struct AppState {
    pub db: Database,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Lazy::force(&TRACING);

    let db_settings = DatabaseSettings::default();
    let db = Database::new(&db_settings).await?;

    let app = Router::new()
        .merge(api::person_routes())
        .merge(api::person_query_routes())
        .route("/health_check", get(health_check))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &hyper::Request<Body>| {
                let uuid = Uuid::new_v4();
                tracing::info_span!(
                    "request",
                    uuid = %uuid,
                    method = %request.method(),
                    uri = %request.uri(),
                )
            }),
        )
        .with_state(db.get_connection());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    info!("Listening on {}", addr);
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument(name = "health check")]
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
