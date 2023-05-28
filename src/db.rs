use surrealdb::sql::Statement;

use color_eyre::Result;
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
            true => Surreal::new::<Wss>(connection_string).await?,
            false => Surreal::new::<Ws>(connection_string).await?,
        };

        client
            .signin(Root {
                username: &configuration.username,
                password: &configuration.password,
            })
            .await?;

        client
            .use_ns(&configuration.namespace)
            .use_db(&configuration.database)
            .await?;
        Ok(Self { client })
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

    pub fn add_query(&mut self, query: &str) {
        self.queries.push(query.to_string());
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
}

pub struct Transaction(pub String);

impl AsRef<str> for Transaction {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl IntoQuery for Transaction {
    fn into_query(self) -> std::result::Result<Vec<Statement>, surrealdb::Error> {
        sql::parse(self.as_ref())?.into_query()
    }
}
