use crate::error::Error;
use axum::extract::{Path, State};
use axum::{Json, Router};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use surrealdb::{engine::remote::ws::Client, Surreal};

const PERSON: &str = "person";

pub fn person_routes() -> Router<Surreal<Client>> {
    Router::new()
        .route("/person/:id", axum::routing::post(create))
        .route("/person/:id", axum::routing::get(read))
        .route("/person/:id", axum::routing::put(update))
        .route("/person/:id", axum::routing::delete(delete))
        .route("/people", axum::routing::get(list))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    name: String,
}

#[debug_handler]
#[tracing::instrument(name = "Create", skip(db, id, person))]
pub async fn create(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = db.create((PERSON, &*id)).content(person).await?;
    Ok(Json(person))
}

#[tracing::instrument(name = "Read", skip(db, id))]
pub async fn read(
    State(db): State<Surreal<Client>>,
    id: Path<String>
) -> Result<Json<Option<Person>>, Error> {
    let person = db.select((PERSON, &*id)).await?;
    Ok(Json(person))
}

#[tracing::instrument(name = "Update", skip(db, id, person))]
pub async fn update(
    State(db): State<Surreal<Client>>,
    id: Path<String>,
    Json(person): Json<Person>,
) -> Result<Json<Option<Person>>, Error> {
    let person = db.update((PERSON, &*id)).content(person).await?;
    Ok(Json(person))
}

#[tracing::instrument(name = "Delete", skip(db, id))]
pub async fn delete(
    State(db): State<Surreal<Client>>,
    id: Path<String>
) -> Result<Json<Option<Person>>, Error> {
    let person = db.delete((PERSON, &*id)).await?;
    Ok(Json(person))
}

#[tracing::instrument(name = "List", skip(db))]
pub async fn list(State(db): State<Surreal<Client>>) -> Result<Json<Vec<Person>>, Error> {
    let people = db.select(PERSON).await?;
    Ok(Json(people))
}
