use crate::db::QueryManager;
use crate::error::Error;
use axum::extract::{Path, State};
use axum::{Json, Router};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use surrealdb::{engine::remote::ws::Client, Surreal};

const PERSON: &str = "person";

pub fn person_query_routes() -> Router<Surreal<Client>> {
    Router::new()
        .route("/person/qry/:id", axum::routing::post(create))
        .route("/person/qry/:id", axum::routing::get(read))
        .route("/person/qry/:id", axum::routing::put(update))
        .route("/person/qry/:id", axum::routing::delete(delete))
        .route("/person/qry/people", axum::routing::get(list))
        .route("/person/qry/batch_up", axum::routing::post(batch_up))
        .route("/person/qry/batch_down", axum::routing::delete(batch_down))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    name: String,
}

#[debug_handler]
#[tracing::instrument(name = "Batch Delete", skip(db))]
pub async fn batch_down(
    State(db): State<Surreal<Client>>,
) -> Result<Json<Option<Vec<Person>>>, Error> {
    let sql = format!("DELETE {}", PERSON);
    tracing::info!(sql);
    let people: Option<Vec<Person>> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(Json(people))
}

#[debug_handler]
#[tracing::instrument(name = "Batch Create", skip(db, people))]
pub async fn batch_up(
    State(db): State<Surreal<Client>>,
    Json(people): Json<Vec<Person>>,
) -> Result<Json<Option<Vec<Person>>>, Error> {
    let people = batch_up_fn(&db, people).await?;
    Ok(Json(Some(people)))
}

async fn batch_up_fn(
    db: &Surreal<Client>,
    people: Vec<Person>,
) -> Result<Vec<Person>, Error> {
    let mut manager = QueryManager::new();
    for person in people {
        let sql = format!("CREATE person:uuid() CONTENT {{ name: '{}' }}", person.name);
        manager.add_query(&sql).await.unwrap();
    }
    let _results = manager.execute(db).await.unwrap();
    let sql = format!("SELECT * FROM {}", PERSON);
    tracing::info!(sql);
    let people: Vec<Person> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(people)
}

#[debug_handler]
#[tracing::instrument(name = "Create", skip(db, id, person))]
pub async fn create(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = create_person(&db, &id, person).await?;
    Ok(Json(person))
}

#[debug_handler]
#[tracing::instrument(name = "Read", skip(db, id))]
pub async fn read(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
) -> Result<Json<Option<Person>>, Error> {
    let person = read_person(&db, &id).await?;
    Ok(Json(person))
}

#[debug_handler]
#[tracing::instrument(name = "Update", skip(db, id, person))]
pub async fn update(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = update_person(&db, &id, person).await?;
    Ok(Json(person))
}

#[debug_handler]
#[tracing::instrument(name = "Delete", skip(db, id))]
pub async fn delete(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
) -> Result<Json<Option<Person>>, Error> {
    let person = delete_person(&db, &id).await?;
    Ok(Json(person))
}

#[debug_handler]
#[tracing::instrument(name = "List", skip(db))]
pub async fn list(State(db): State<Surreal<Client>>) -> Result<Json<Vec<Person>>, Error> {
    let people = list_people(&db).await?;
    Ok(Json(people))
}

#[tracing::instrument(name = "Query: Create Person", skip(db, id, person))]
async fn create_person(
    db: &Surreal<Client>,
    id: &str,
    person: Person,
) -> Result<Option<Person>, Error> {
    let sql = format!(
        "CREATE {} CONTENT {{ name: '{}' }}",
        Thing::from((PERSON, id)),
        person.name
    );
    tracing::info!(sql);
    let person: Option<Person> = db.query(sql).await.unwrap().take(0).unwrap();

    Ok(person)
}

#[tracing::instrument(name = "Query: Read Person", skip(db, id))]
async fn read_person(db: &Surreal<Client>, id: &str) -> Result<Option<Person>, Error> {
    let sql = format!(
        "SELECT * FROM {} WHERE id = '{}'",
        PERSON,
        Thing::from((PERSON, id)),
    );
    tracing::info!(sql);
    let person: Option<Person> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(person)
}

#[tracing::instrument(name = "Query: Update Person", skip(db, id, person))]
async fn update_person(
    db: &Surreal<Client>,
    id: &str,
    person: Person,
) -> Result<Option<Person>, Error> {
    let sql = format!(
        "UPDATE {} CONTENT {{ name: '{}' }}",
        Thing::from((PERSON, id)),
        person.name
    );
    tracing::info!(sql);
    let person: Option<Person> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(person)
}

#[tracing::instrument(name = "Query: Delete Person", skip(db, id))]
async fn delete_person(db: &Surreal<Client>, id: &str) -> Result<Option<Person>, Error> {
    let sql = format!("DELETE {}", Thing::from((PERSON, id)));
    tracing::info!(sql);
    let person: Option<Person> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(person)
}

#[tracing::instrument(name = "Query: List People", skip(db))]
async fn list_people(db: &Surreal<Client>) -> Result<Vec<Person>, Error> {
    let sql = format!("SELECT * FROM {}", PERSON);
    tracing::info!(sql);
    let people: Vec<Person> = db.query(sql).await.unwrap().take(0).unwrap();
    Ok(people)
}
