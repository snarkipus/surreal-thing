use crate::error::Error;
use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

const PERSON: &str = "person";

type Db = State<Surreal<Client>>;

#[derive(Serialize, Deserialize)]
pub struct Person {
    name: String,
}

pub async fn create(
    db: Db,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = db.create((PERSON, &*id)).content(person).await?;
    Ok(Json(person))
}

pub async fn read(db: Db, id: Path<String>) -> Result<Json<Option<Person>>, Error> {
    let person = db.select((PERSON, &*id)).await?;
    Ok(Json(person))
}

pub async fn update(
    db: Db,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = db.update((PERSON, &*id)).content(person).await?;
    Ok(Json(person))
}

pub async fn delete(db: Db, id: Path<String>) -> Result<Json<Option<Person>>, Error> {
    let person = db.delete((PERSON, &*id)).await?;
    Ok(Json(person))
}

pub async fn list(db: Db) -> Result<Json<Vec<Person>>, Error> {
    let people = db.select(PERSON).await?;
    Ok(Json(people))
}

#[cfg(test)]
mod tests {
    use surrealdb::{
        engine::remote::ws::Ws,
        opt::auth::Root,
        sql::{Id, Thing},
    };

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct PersonModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<Thing>,
        name: String,
    }

    async fn create_db() -> Surreal<Client> {
        let db = Surreal::new::<Ws>("localhost:8000").await.unwrap();
        db.signin(Root {
            username: "surreal",
            password: "password",
        })
        .await
        .unwrap();
        db.use_ns("namespace").use_db("database").await.unwrap();

        db
    }

    #[tokio::test]
    async fn create_person() {
        let db = create_db().await;

        // Simple stuff - SurrealDB handles creating the uuid() in the database
        let sql = "CREATE person:uuid() CONTENT { name: $name }";

        let mut res = db.query(sql).bind(("name", "Blaze")).await.unwrap();

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

    #[derive(Debug, Serialize, Deserialize)]
    struct LicenseModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<Thing>,
        license_number: usize,
    }

    #[tokio::test]
    async fn create_license() {
        // region: working code
        let db = create_db().await;

        // create license number
        let license_number: usize = 12345;

        // create new person: Doc McStuffins
        let sql = "CREATE person:uuid() CONTENT { name: $name }";
        let mut res = db.query(sql).bind(("name", "McStuffins")).await.unwrap();

        let _person_id = res
            .take(0)
            .map(|p: Option<PersonModel>| p.unwrap())
            .map(|p: PersonModel| p.id.unwrap())
            .map(|t: Thing| t.id)
            .map(|id: Id| id.to_raw())
            .unwrap();

        // create new license
        let sql = "CREATE registry:uuid() CONTENT { registration: $license_number }";

        let _res = db
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

        let _res = db
            .query(sql)
            .bind(("name", "McStuffins"))
            .bind(("license_number", license_number))
            .await
            .unwrap();

        // create another license number
        let license_number: usize = 678910;

        // create another ew license
        let sql = "CREATE registry:uuid() CONTENT { registration: $license_number }";

        let _res = db
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

        let _res = db
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

        let mut res = db
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
}
