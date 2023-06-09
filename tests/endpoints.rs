use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
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

// region: -- helper trait for printing httpc responses
trait SexyPrint {
    fn sexy_print(&self, method: &str, url: &str) -> color_eyre::Result<()>;
}

// format shamelessly stolen from httpc-test
// repo: https://github.com/jeremychone/rust-httpc-test/blob/main/src/response.rs#L72
impl SexyPrint for minreq::Response {
    fn sexy_print(&self, method: &str, url: &str) -> color_eyre::Result<()> {
        println!("\n=== Response for {} {}", method, url);
        println!("=> {:<15}: {}", "Status", self.status_code);
        println!("=> {:<15}:", "Headers");
        for (n, v) in self.headers.iter() {
            println!("   {n}: {v:?}");
        }
        println!("=> {:<15}:", "Response Body");
        println!("{:?}\n", self.as_str());
        Ok(())
    }
}
// endregion: -- helper trait for printing httpc responses

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Person {
    name: String,
}

#[tokio::test]
async fn crud_endpoints_work() -> color_eyre::Result<()> {
    Lazy::force(&TRACING);

    // Arrange
    let conn_string = format!("http://{}:{}", "127.0.0.1", "8080");

    // Act

    // HEALTH_CHECK: GET -> .route("/health_check", get(health_check))
    let route = "/health_check";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // CREATE: POST -> .route("/person/:id", post(person::create))
    let route = "/person/1";
    let data: Person = Person {
        name: "John".into(),
    };
    let response = minreq::post(format!("{conn_string}{route}"))
        .with_json(&data)?
        .send()?;
    response.sexy_print("POST", format!("{conn_string}{route}").as_str())?;

    // READ: GET -> .route("/person/:id", get(person::read))
    let route = "/person/1";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // UPDATE: PUT -> .route("/person/:id", put(person::update))
    let route = "/person/1";
    let data: Person = Person {
        name: "Mark".into(),
    };
    let response = minreq::put(format!("{conn_string}{route}"))
        .with_json(&data)?
        .send()?;
    response.sexy_print("PUT", format!("{conn_string}{route}").as_str())?;

    // DELETE: DELETE -> .route("/person/:id", delete(person::delete))
    let route = "/person/1";
    let response = minreq::delete(format!("{conn_string}{route}"))
        .send()
        .unwrap();
    response.sexy_print("DELETE", format!("{conn_string}{route}").as_str())?;

    // LIST: GET -> .route("/people", get(person::list))
    let route = "/people";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // Assert

    Ok(())
}

#[tokio::test]
async fn crud_query_endpoints_work() -> color_eyre::Result<()> {
    Lazy::force(&TRACING);

    // Arrange
    let conn_string = format!("http://{}:{}", "127.0.0.1", "8080");

    // Act

    // HEALTH_CHECK: GET -> .route("/health_check", get(health_check))
    let route = "/health_check";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // CREATE: POST -> .route("/person/:id", post(person::create))
    let route = "/person/qry/1";
    let data: Person = Person {
        name: "John".into(),
    };
    let response = minreq::post(format!("{conn_string}{route}"))
        .with_json(&data)?
        .send()?;
    response.sexy_print("POST", format!("{conn_string}{route}").as_str())?;

    // READ: GET -> .route("/person/:id", get(person::read))
    let route = "/person/qry/1";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // UPDATE: PUT -> .route("/person/:id", put(person::update))
    let route = "/person/qry/1";
    let data: Person = Person {
        name: "Mark".into(),
    };
    let response = minreq::put(format!("{conn_string}{route}"))
        .with_json(&data)?
        .send()?;
    response.sexy_print("PUT", format!("{conn_string}{route}").as_str())?;

    // LIST: GET -> .route("/people", get(person::list))
    let route = "/person/qry/people";
    let response = minreq::get(format!("{conn_string}{route}")).send().unwrap();
    response.sexy_print("GET", format!("{conn_string}{route}").as_str())?;

    // DELETE: DELETE -> .route("/person/:id", delete(person::delete))
    let route = "/person/qry/1";
    let response = minreq::delete(format!("{conn_string}{route}"))
        .send()
        .unwrap();
    response.sexy_print("DELETE", format!("{conn_string}{route}").as_str())?;

    // BATCH: POST -> .route("/person/qry/batch", post(person::batch))
    let route = "/person/qry/batch_up";
    let data: Vec<Person> = vec![
        Person {
            name: "Luke".into(),
        },
        Person {
            name: "John".into(),
        },
    ];
    let response = minreq::post(format!("{conn_string}{route}"))
        .with_json(&data)?
        .send()?;
    response.sexy_print("POST", format!("{conn_string}{route}").as_str())?;

    // DELETE: DELETE -> .route("/person/qry/batch_down", delete(person::delete))
    let route = "/person/qry/batch_down";
    let response = minreq::delete(format!("{conn_string}{route}"))
        .send()
        .unwrap();
    response.sexy_print("DELETE", format!("{conn_string}{route}").as_str())?;

    Ok(())
}
