use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use surrealdb::{engine::remote::ws::Client, sql::Thing, Surreal};

use surreal_simple::{
    surreal::db::{Database, DatabaseSettings, Transaction},
    telemetry::{get_subscriber, init_subscriber},
};
use uuid::Uuid;
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

pub struct TestApp {
    pub db: Surreal<Client>,
}

async fn setup() -> TestApp {
    Lazy::force(&TRACING);

    let db = Database::new(&DatabaseSettings::default()).await.unwrap();

    TestApp {
        db: db.client,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersonModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    name: String,
}

#[tokio::test]
#[serial]
async fn create_person() {
    // Arrange
    let app = setup().await;
    let id = Thing::from(("person".to_string(), Uuid::new_v4().to_string()));
    let sql = format!("CREATE {} CONTENT {{ name: $name }}", id);

    // Act
    let mut res = app.db.query(sql).bind(("name", "Blaze")).await.unwrap();
    let res_id: Option<Thing> = res.take((0, "id")).unwrap();

    // Assert
    assert_eq!(res_id.unwrap(), id);

    // Teardown
    let sql = format!("DELETE {}", id);
    let _ = app.db.query(sql).await;
}

#[tokio::test]
#[serial]
async fn create_people() {
    // Arrange
    let app = setup().await;
    let sql = "
            BEGIN TRANSACTION;
            CREATE person:uuid() CONTENT { name: $name1 };
            CREATE person:uuid() CONTENT { name: $name2 };
            CREATE person:uuid() CONTENT { name: $name3 };
            COMMIT TRANSACTION;
        ";

    // Act
    let mut res = app
        .db
        .query(sql)
        .bind(("name1", "foo"))
        .bind(("name2", "bar"))
        .bind(("name3", "baz"))
        .await
        .unwrap();

    let person_0: Option<PersonModel> = res.take(0).unwrap();
    let person_1: Option<PersonModel> = res.take(1).unwrap();
    let person_2: Option<PersonModel> = res.take(2).unwrap();

    // Assert
    assert_eq!(person_0.unwrap().name, "foo");
    assert_eq!(person_1.unwrap().name, "bar");
    assert_eq!(person_2.unwrap().name, "baz");

    // Teardown
    let sql = "DELETE person WHERE name = 'foo' OR name = 'bar' OR name = 'baz'";
    let _ = app.db.query(sql).await;
}

#[tokio::test]
#[serial]
async fn create_transaction() {
    // Arrange
    let app = setup().await;
    let transaction = Transaction::begin(&app.db).await.unwrap();
    let conn = transaction.conn;
    let sql_0 = format!(
        "CREATE {} CONTENT {{ name: 'foo' }}",
        Thing::from(("person".into(), Uuid::new_v4().to_string()))
    );
    let sql_1 = format!(
        "CREATE {} CONTENT {{ name: 'bar' }}",
        Thing::from(("person".into(), Uuid::new_v4().to_string()))
    );
    let sql_2 = format!(
        "CREATE {} CONTENT {{ name: 'baz' }}",
        Thing::from(("person".into(), Uuid::new_v4().to_string()))
    );

    // Act
    conn.query(&sql_0).await.unwrap();
    conn.query(&sql_1).await.unwrap();
    conn.query(&sql_2).await.unwrap();
    transaction.commit().await;

    // Assert
    let sql = "SELECT * FROM person ORDER BY name ASC";
    let mut res = app.db.query(sql).await.unwrap();
    let people: Vec<PersonModel> = res.take(0).unwrap();
    let names = vec!["bar", "baz", "foo"];
    for (i, person) in people.iter().enumerate() {
        assert_eq!(person.name, names[i]);
    }

    // Teardown
    let sql = "DELETE person WHERE name = 'foo' OR name = 'bar' OR name = 'baz'";
    let _ = app.db.query(sql).await;
}

#[derive(Debug, Serialize, Deserialize)]
struct LicenseModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    license_number: usize,
}

#[tokio::test]
#[serial]
async fn create_license() {
    // region: Arrange
    let app = setup().await;
    let transaction = Transaction::begin(&app.db).await.unwrap();
    let conn = transaction.conn;

    // Create Doc McStuffins
    let doc_id = Thing::from(("person".to_string(), Uuid::new_v4().to_string()));
    let sql = format!("CREATE {} CONTENT {{ name: '{}' }}", doc_id, "McStuffins");
    conn.query(&sql).await.unwrap();

    // Create a license for Doc McStuffins
    let license_number_0: usize = 12345;
    let lic_id_0 = Thing::from(("registry".to_string(), Uuid::new_v4().to_string()));
    let sql = format!(
        "CREATE {} CONTENT {{ registration: {} }}",
        lic_id_0, license_number_0
    );
    conn.query(&sql).await.unwrap();

    // Create another license for Doc McStuffins
    let license_number_1: usize = 678910;
    let lic_id_1 = Thing::from(("registry".to_string(), Uuid::new_v4().to_string()));
    let sql = format!(
        "CREATE {} CONTENT {{ registration: {} }}",
        lic_id_1, license_number_1
    );
    conn.query(&sql).await.unwrap();

    transaction.commit().await;

    // endregion

    // region: Act
    let sql = "
            RELATE $license->licenses->$person SET id = licenses:uuid();
        ";
    let _ = app
        .db
        .query(sql)
        .bind(("license", &lic_id_0))
        .bind(("person", &doc_id))
        .await
        .unwrap();

    let sql = "
        RELATE $license->licenses->$person SET id = licenses:uuid();
    ";
    let _ = app
        .db
        .query(sql)
        .bind(("license", &lic_id_1))
        .bind(("person", &doc_id))
        .await
        .unwrap();
    // endregion

    // region: Assert
    let sql = "SELECT name, ->licenses->person.name AS name FROM ( SELECT id FROM registry WHERE registration = $registration );";
    let mut res = app
        .db
        .query(sql)
        .bind(("registration", license_number_0))
        .await
        .unwrap();

    let name: Option<Vec<String>> = res.take((0, "name")).unwrap();
    assert_eq!(name.unwrap(), vec!["McStuffins"]);

    let sql = "SELECT name, ->licenses->person.name AS name FROM ( SELECT id FROM registry WHERE registration = $registration );";
    let mut res = app
        .db
        .query(sql)
        .bind(("registration", license_number_1))
        .await
        .unwrap();

    let name: Option<Vec<String>> = res.take((0, "name")).unwrap();
    assert_eq!(name.unwrap(), vec!["McStuffins"]);

    // SELECT registration, <-licenses<-registry.registration AS registration FROM (SELECT id FROM person WHERE name='McStuffins')
    let sql = "SELECT registrations, <-licenses<-registry.registration AS registrations FROM (SELECT id FROM person WHERE name=$name);";
    let mut res = app
        .db
        .query(sql)
        .bind(("name", "McStuffins"))
        .await
        .unwrap();

    let registrations: Option<Vec<usize>> = res.take((0, "registrations")).unwrap();
    for registration in registrations.unwrap() {
        assert!(registration == license_number_0 || registration == license_number_1);
    }

    // Teardown
    let transaction = Transaction::begin(&app.db).await.unwrap();
    let conn = transaction.conn;
    let sql = "DELETE person WHERE name = 'McStuffins'";
    conn.query(sql).await.unwrap();
    let sql = "DELETE registry WHERE registration = 12345 OR registration = 678910";
    conn.query(sql).await.unwrap();
    let sql = "DELETE licenses";
    conn.query(sql).await.unwrap();
    transaction.commit().await;
    // endregion
}
