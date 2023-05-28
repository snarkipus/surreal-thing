use once_cell::sync::Lazy;
use serde_json::json;

use surreal_simple::telemetry::{get_subscriber, init_subscriber};

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

#[tokio::test]
async fn crud_endpoints_work() -> color_eyre::Result<()> {
    Lazy::force(&TRACING);

    // Arrange
    let conn_string = format!("http://{}:{}", "127.0.0.1", "8080");
    let hc = httpc_test::new_client(&conn_string)?;

    // Act

    // CREATE: POST -> .route("/person/:id", post(person::create))
    hc.do_post("/person/1", json!({"name": "John"}))
        .await?
        .print()
        .await?;

    // READ: GET -> .route("/person/:id", get(person::read))
    hc.do_get("/person/1").await?.print().await?;

    // UPDATE: PUT -> .route("/person/:id", put(person::update))
    hc.do_put("/person/1", json!({"name": "Mark"}))
        .await?
        .print()
        .await?;

    // DELETE: DELETE -> .route("/person/:id", delete(person::delete))
    hc.do_delete("/person/1").await?.print().await?;

    // LIST: GET -> .route("/people", get(person::list))
    hc.do_get("/people").await?.print().await?;

    // HEALTH_CHECK: GET -> .route("/health_check", get(health_check))
    hc.do_get("/health_check").await?.print().await?;

    // Assert

    Ok(())
}
