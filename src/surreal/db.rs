use crate::error::Error;
use color_eyre::{eyre::Context, Result};
use futures_core::future::BoxFuture;

use surrealdb::{
    engine::remote::ws::{Client, Ws, Wss},
    opt::auth::Root,
    Surreal,
};

// region: -- DatabaseSettings
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub database: String,
    pub ssl_mode: bool,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 8000,
            username: "surreal".into(),
            password: "password".into(),
            namespace: "namespace".into(),
            database: "database".into(),
            ssl_mode: false,
        }
    }
}
// endregion: -- DatabaseSettings

// region: -- Database
#[derive(Clone, Debug)]
pub struct Database {
    pub client: Surreal<Client>,
}

impl Database {
    // region: -- SurrealDB Initialization
    #[tracing::instrument(
        name = "Creating new SurrealDB Client",
        skip(configuration),
        fields(
            db = %configuration.database
        )
      )]
    pub async fn new(configuration: &DatabaseSettings) -> Result<Self> {
        let connection_string = format!("{}:{}", configuration.host, configuration.port);

        let client = match configuration.ssl_mode {
            true => Surreal::new::<Wss>(connection_string)
                .await
                .context("Failed to make Wss connection")?,
            false => Surreal::new::<Ws>(connection_string)
                .await
                .context("Failed to make Ws connection")?,
        };

        client
            .signin(Root {
                username: &configuration.username,
                password: &configuration.password,
            })
            .await
            .context("Failed to Sign-In")?;

        client
            .use_ns(&configuration.namespace)
            .use_db(&configuration.database)
            .await
            .context("Failed to set namespace & database")?;

        Ok(Self { client })
    }
}
// endregion: -- Database

// region: -- Transaction
pub struct Transaction<'c> {
    pub conn: &'c Surreal<Client>,
    pub open: bool,
}

impl<'c> Transaction<'c> {
    pub fn begin(conn: &'c Surreal<Client>) -> BoxFuture<'c, Result<Self, Error>> {
        Box::pin(async move {
            let sql = "BEGIN TRANSACTION;".to_string();
            let response = conn.query(sql).await?;
            response.check()?;

            Ok(Self { conn, open: true })
        })
    }

    pub async fn commit(mut self) -> BoxFuture<'c, Result<(), Error>> {
        Box::pin(async move {
            let sql = "COMMIT TRANSACTION;";
            let response = self.conn.query(sql).await?;
            response.check()?;
            self.open = false;

            Ok(())
        })
    }

    pub async fn rollback(mut self) -> BoxFuture<'c, Result<(), Error>> {
        Box::pin(async move {
            let sql = "CANCEL TRANSACTION;";
            let response = self.conn.query(sql).await?;
            response.check()?;
            self.open = false;
            Ok(())
        })
    }
}
// endregion: -- Transaction