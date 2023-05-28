use surrealdb::sql::Statement;

use color_eyre::{eyre::Context, Result};
use surrealdb::{
    engine::remote::ws::{Client, Ws, Wss},
    opt::{auth::Root, IntoQuery},
    sql, Surreal,
};

// region: -- DatabaseSettings
#[derive(Clone, Debug)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub database: String,
    pub require_ssl: bool,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: "8000".into(),
            username: "surreal".into(),
            password: "password".into(),
            namespace: "namespace".into(),
            database: "database".into(),
            require_ssl: false,
        }
    }
}
// endregion: -- DatabaseSettings

// region: -- Database
#[derive(Clone, Debug)]
pub struct Database {
    pub client: Surreal<Client>,
    pub query_manager: QueryManager,
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

        let client = match configuration.require_ssl {
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

        Ok(Self {
            client,
            query_manager: QueryManager::new(),
        })
    }
    // endregion: --- SurrealDB Initialization

    // region:: -- Get Connection
    pub fn get_connection(&self) -> Surreal<Client> {
        self.client.clone()
    }
    // endregion:: -- Get Connection
}
// endregion: -- Database

// region: -- Query Manager
#[derive(Clone, Debug, Default)]
pub struct QueryManager {
    pub queries: Vec<String>,
}

impl QueryManager {
    pub fn new() -> QueryManager {
        QueryManager {
            queries: Vec::new(),
        }
    }

    #[tracing::instrument(
        name = "Adding query to QueryManager",
        skip(self, query),
        fields(
            query = %query
        )
    )]
    pub fn add_query(&mut self, query: &str) -> Result<()> {
        let query = sql::parse(query).context("Failed to parse query")?;
        self.queries.push(query.to_string());
        Ok(())
    }

    pub fn generate_transaction(&self) -> Transaction {
        let mut transaction = String::from("BEGIN TRANSACTION;\n");
        for query in &self.queries {
            transaction.push_str(query);
            transaction.push_str(";\n");
        }
        transaction.push_str("COMMIT TRANSACTION;");
        Transaction(transaction)
    }

    #[tracing::instrument(name = "Executing QueryManager", skip(self, db))]
    pub async fn execute(&mut self, db: &Surreal<Client>) -> Result<()> {
        let transaction = self.generate_transaction();
        match db.query(transaction).await {
            Ok(_) => {
                self.queries.clear();
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

pub struct Transaction(pub String);

impl AsRef<str> for Transaction {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl IntoQuery for Transaction {
    fn into_query(self) -> Result<Vec<Statement>, surrealdb::Error> {
        sql::parse(self.as_ref())?.into_query()
    }
}
