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
    use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, sql::Thing};

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
    
        let mut res = db
            .query(sql)
            .bind(("name", "Blaze"))            
            .await
            .unwrap();
        
        // This just sucks - so much unwrapping 
        let person: PersonModel = res.take(0)
            .map(|p: Option<PersonModel>| p.unwrap())
            .unwrap();
        
        // This is actually a Thing
        let id = &person.id.unwrap();
        let name = &person.name;
        assert_eq!(id.tb, "person");
        assert_eq!(name, "Blaze");
        println!("id: {}", &id.id.to_raw());
    }
}
