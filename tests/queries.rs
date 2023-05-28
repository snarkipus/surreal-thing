use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::remote::ws::Client,
    sql::{Id, Thing},
    Surreal,
};

use surreal_simple::db::{Database, DatabaseSettings, QueryManager};

pub struct TestApp {
    pub db: Surreal<Client>,
    pub manager: QueryManager,
}

async fn setup() -> TestApp {
    let db = Database::new(&DatabaseSettings::default()).await.unwrap();
    
    
    TestApp {
        db: db.get_connection(),
        manager: QueryManager::new(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersonModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    name: String,
}

#[tokio::test]
async fn create_person() {
    let app = setup().await;
    
    // Simple stuff - SurrealDB handles creating the uuid() in the database
    let sql = "CREATE person:uuid() CONTENT { name: $name }";

    let mut res = app.db.query(sql).bind(("name", "Blaze")).await.unwrap();

    // This just sucks - so much unwrapping
    let person: PersonModel = res
        .take(0)
        .map(|p: Option<PersonModel>| p.unwrap())
        .unwrap();

    // This is actually a Thing
    let id = &person.id.unwrap();
    let name = &person.name;
    assert_eq!(id.tb, "person");
    assert_eq!(name, "Blaze");
    println!("id: {}", &id.id.to_raw());
}

#[tokio::test]
async fn create_people() {
    let app = setup().await;

    // Simple stuff - SurrealDB handles creating the uuid() in the database
    let sql = "
            BEGIN TRANSACTION;
            CREATE person:uuid() CONTENT { name: $name1 };
            CREATE person:uuid() CONTENT { name: $name2 };
            CREATE person:uuid() CONTENT { name: $name3 };
            COMMIT TRANSACTION;
        ";

    let mut res = app.db
        .query(sql)
        .bind(("name1", "foo"))
        .bind(("name2", "bar"))
        .bind(("name3", "baz"))
        .await
        .unwrap();

    // This just sucks - so much unwrapping
    let person: PersonModel = res
        .take(0)
        .map(|p: Option<PersonModel>| p.unwrap())
        .unwrap();

    // This is actually a Thing
    let id = &person.id.unwrap();
    let name = &person.name;
    assert_eq!(id.tb, "person");
    assert_eq!(name, "foo");
    println!("id: {}", &id.id.to_raw());
}

#[tokio::test]
async fn create_transaction() {
    let mut app = setup().await;

    let sql_0 = "CREATE person:uuid() CONTENT { name: 'foo' }";
    app.manager.add_query(sql_0).unwrap();

    let sql_1 = "CREATE person:uuid() CONTENT { name: 'bar' }";
    app.manager.add_query(sql_1).unwrap();

    let sql_2 = "CREATE person:uuid() CONTENT { name: 'baz' }";
    app.manager.add_query(sql_2).unwrap();

    let transaction = app.manager.generate_transaction();
    let _res = app.db.query(transaction).await.unwrap();
    
    let sql = "SELECT * FROM person ORDER BY name ASC";
    let mut res = app.db.query(sql).await.unwrap();
    
    let people: Vec<PersonModel> = res.take(0).unwrap();

    let names = vec!["bar", "baz", "foo"];
    for (i, person) in people.iter().enumerate() {
        assert_eq!(person.name, names[i]);        
    }

}

#[derive(Debug, Serialize, Deserialize)]
struct LicenseModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    license_number: usize,
}

#[tokio::test]
async fn create_license() {
    // region: working code
    let app = setup().await;

    // create license number
    let license_number: usize = 12345;

    // create new person: Doc McStuffins
    let sql = "CREATE person:uuid() CONTENT { name: $name }";
    let mut res = app.db.query(sql).bind(("name", "McStuffins")).await.unwrap();

    let _person_id = res
        .take(0)
        .map(|p: Option<PersonModel>| p.unwrap())
        .map(|p: PersonModel| p.id.unwrap())
        .map(|t: Thing| t.id)
        .map(|id: Id| id.to_raw())
        .unwrap();

    // create new license
    let sql = "CREATE registry:uuid() CONTENT { registration: $license_number }";

    let _res = app.db
        .query(sql)
        .bind(("license_number", license_number))
        .await
        .unwrap();

    // relate license to person
    let sql = "
            LET $foo = SELECT id FROM person WHERE name = $name;
            LET $bar = SELECT id FROM registry WHERE registration = $license_number;
            RELATE $bar->licenses->$foo SET id = licenses:uuid();
        ";

    let _res = app.db
        .query(sql)
        .bind(("name", "McStuffins"))
        .bind(("license_number", license_number))
        .await
        .unwrap();

    // create another license number
    let license_number: usize = 678910;

    // create another ew license
    let sql = "CREATE registry:uuid() CONTENT { registration: $license_number }";

    let _res = app.db
        .query(sql)
        .bind(("license_number", &license_number))
        .await
        .unwrap();

    // relate another license to same person
    let sql = "
            LET $foo = SELECT id FROM person WHERE name = $name;
            LET $bar = SELECT id FROM registry WHERE registration = $license_number;
            RELATE $bar->licenses->$foo SET id = licenses:uuid();
        ";

    let _res = app.db
        .query(sql)
        .bind(("name", "McStuffins"))
        .bind(("license_number", &license_number))
        .await
        .unwrap();
    // endregion: working code

    // Select id from person given a license number
    let sql = "
            LET $blah = SELECT id FROM registry WHERE registration = $license_number;
            SELECT *, $blah->licenses->person from person;
        ";

    let mut res = app.db
        .query(sql)
        .bind(("license_number", license_number))
        .await
        .unwrap();

    let person_id: Thing = res
        .take::<Vec<PersonModel>>(1)
        .map(|mut v: Vec<PersonModel>| v.pop())
        .map(|p: Option<PersonModel>| p.unwrap())
        .map(|p: PersonModel| p.id)
        .map(|t: Option<Thing>| t.unwrap())
        .unwrap();

    dbg!(person_id);
}
