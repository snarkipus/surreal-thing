use axum::body::Body;
use once_cell::sync::Lazy;
use telemetry::{get_subscriber, init_subscriber};
use tower_http::trace::TraceLayer;
use tracing::info;

mod error;
mod person;
mod telemetry;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Router, Server};
use uuid::Uuid;
use std::net::SocketAddr;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Lazy::force(&TRACING);

    let db = Surreal::new::<Ws>("localhost:8000").await?;

    db.signin(Root {
        username: "surreal",
        password: "password",
    })
    .await?;

    db.use_ns("namespace").use_db("database").await?;

    let app = Router::new()
        .route("/person/:id", post(person::create))
        .route("/person/:id", get(person::read))
        .route("/person/:id", put(person::update))
        .route("/person/:id", delete(person::delete))
        .route("/people", get(person::list))
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
        .with_state(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    info!("Listening on {}", addr);
    Server::bind(&addr).serve(app.into_make_service()).await?;
    
    Ok(())
}

#[tracing::instrument(
    name = "health check",
)]
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
