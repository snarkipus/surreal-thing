# Database Implementation

##  `sqlx` 

### Database Trait Implementation
[LINK](https://docs.rs/sqlx/latest/sqlx/database/trait.Database.html)

```rust
pub trait Database:
    'static
    + Sized
    + Send
    + Debug
    + for<'r> HasValueRef<'r, Database = Self>
    + for<'q> HasArguments<'q, Database = Self>
    + for<'q> HasStatement<'q, Database = Self>
{
    type Connection: Connection<Database = Self>;

    type TransactionManager: TransactionManager<Database = Self>; // <-- this
    
    type Row: Row<Database = Self>;
    
    type QueryResult: 'static + Sized + Send + Sync + Default + Extend<Self::QueryResult>;
    
    type Column: Column<Database = Self>;
    
    type TypeInfo: TypeInfo;
    
    type Value: Value<Database = Self> + 'static;
}
```

#### PostgreSQL Database Driver Implementation
[LINK](https://docs.rs/sqlx/latest/sqlx/postgres/struct.Postgres.html)
```rust
pub struct Postgres;

impl Database for Postgres {
    type Connection = PgConnection;

    type TransactionManager = PgTransactionManager; // <-- this

    type Row = PgRow;

    type QueryResult = PgQueryResult;

    type Column = PgColumn;

    type TypeInfo = PgTypeInfo;

    type Value = PgValue;
}

impl<'r> HasValueRef<'r> for Postgres {
    type Database = Postgres;

    type ValueRef = PgValueRef<'r>;
}

impl HasArguments<'_> for Postgres {
    type Database = Postgres;

    type Arguments = PgArguments;

    type ArgumentBuffer = PgArgumentBuffer;
}

impl<'q> HasStatement<'q> for Postgres {
    type Database = Postgres;

    type Statement = PgStatement<'q>;
}

impl HasStatementCache for Postgres {}
```
### `TransactionManager` Trait
[LINK](https://docs.rs/sqlx-core/0.6.3/src/sqlx_core/transaction.rs.html#15-35)
```rust 
pub trait TransactionManager {
    type Database: Database;

    fn begin(
        conn: &mut <Self::Database as Database>::Connection,
    ) -> BoxFuture<'_, Result<(), Error>>;

    fn commit(
        conn: &mut <Self::Database as Database>::Connection,
    ) -> BoxFuture<'_, Result<(), Error>>;

    fn rollback(
        conn: &mut <Self::Database as Database>::Connection,
    ) -> BoxFuture<'_, Result<(), Error>>;

    fn start_rollback(conn: &mut <Self::Database as Database>::Connection);
}
```
#### PostgreSQL TransactionManager
[LINK](https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgTransactionManager.html)
```rust
pub struct PgTransactionManager;

impl TransactionManager for PgTransactionManager {
    type Database = Postgres;

    fn begin(conn: &mut PgConnection) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            conn.execute(&*begin_ansi_transaction_sql(conn.transaction_depth))
                .await?;

            conn.transaction_depth += 1;

            Ok(())
        })
    }

    fn commit(conn: &mut PgConnection) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            if conn.transaction_depth > 0 {
                conn.execute(&*commit_ansi_transaction_sql(conn.transaction_depth))
                    .await?;

                conn.transaction_depth -= 1;
            }

            Ok(())
        })
    }

    fn rollback(conn: &mut PgConnection) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            if conn.transaction_depth > 0 {
                conn.execute(&*rollback_ansi_transaction_sql(conn.transaction_depth))
                    .await?;

                conn.transaction_depth -= 1;
            }

            Ok(())
        })
    }

    fn start_rollback(conn: &mut PgConnection) {
        if conn.transaction_depth > 0 {
            conn.queue_simple_query(&rollback_ansi_transaction_sql(conn.transaction_depth));

            conn.transaction_depth -= 1;
        }
    }
}
```

### `sqlx::Transaction` Struct
[LINK](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html)
```rust
pub struct Transaction<'c, DB>
where
    DB: Database,
{
    connection: MaybePoolConnection<'c, DB>,
    open: bool,
}
```
An in-progress database transaction or savepoint.

A transaction starts with a call to `Pool::begin` or `Connection::begin`.

A transaction should end with a call to `commit` or `rollback`. If neither are called before the transaction goes out-of-scope, `rollback` is called. In other words, `rollback` is called on `drop` if the transaction is still in-progress.

A savepoint is a special mark inside a transaction that allows all commands that are executed after it was established to be rolled back, restoring the transaction state to what it was at the time of the savepoint.

```rust
impl<'c, DB> Transaction<'c, DB>
where
    DB: Database,
{
    pub(crate) fn begin(
        conn: impl Into<MaybePoolConnection<'c, DB>>,
    ) -> BoxFuture<'c, Result<Self, Error>> {
        let mut conn = conn.into();

        Box::pin(async move {
            DB::TransactionManager::begin(&mut conn).await?;

            Ok(Self {
                connection: conn,
                open: true,
            })
        })
    }

    pub async fn commit(mut self) -> Result<(), Error> {
        DB::TransactionManager::commit(&mut self.connection).await?;
        self.open = false;

        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), Error> {
        DB::TransactionManager::rollback(&mut self.connection).await?;
        self.open = false;

        Ok(())
    }
}
```

## `Connection` Trait
[LINK](https://docs.rs/sqlx/latest/sqlx/trait.Connection.html)
```rust
pub trait Connection: Send {
    type Database: Database;

    type Options: ConnectOptions<Connection = Self>;

    fn close(self) -> BoxFuture<'static, Result<(), Error>>;

    fn close_hard(self) -> BoxFuture<'static, Result<(), Error>>;

    fn ping(&mut self) -> BoxFuture<'_, Result<(), Error>>;

    fn begin(&mut self) -> BoxFuture<'_, Result<Transaction<'_, Self::Database>, Error>>
    where
        Self: Sized;

    fn transaction<'a, F, R, E>(&'a mut self, callback: F) -> BoxFuture<'a, Result<R, E>>
    where
        for<'c> F: FnOnce(&'c mut Transaction<'_, Self::Database>) -> BoxFuture<'c, Result<R, E>>
            + 'a
            + Send
            + Sync,
        Self: Sized,
        R: Send,
        E: From<Error> + Send,
    {
        Box::pin(async move {
            let mut transaction = self.begin().await?;
            let ret = callback(&mut transaction).await;

            match ret {
                Ok(ret) => {
                    transaction.commit().await?;

                    Ok(ret)
                }
                Err(err) => {
                    transaction.rollback().await?;

                    Err(err)
                }
            }
        })
    }

    fn cached_statements_size(&self) -> usize
    where
        Self::Database: HasStatementCache,
    {
        0
    }

    fn clear_cached_statements(&mut self) -> BoxFuture<'_, Result<(), Error>>
    where
        Self::Database: HasStatementCache,
    {
        Box::pin(async move { Ok(()) })
    }

    fn flush(&mut self) -> BoxFuture<'_, Result<(), Error>>;

    fn should_flush(&self) -> bool;

    #[inline]
    fn connect(url: &str) -> BoxFuture<'static, Result<Self, Error>>
    where
        Self: Sized,
    {
        let options = url.parse();

        Box::pin(async move { Ok(Self::connect_with(&options?).await?) })
    }

    fn connect_with(options: &Self::Options) -> BoxFuture<'_, Result<Self, Error>>
    where
        Self: Sized,
    {
        options.connect()
    }
}
``` 

### PostgreSQL Connection (`PgConnection`)
[LINK](https://docs.rs/sqlx/latest/sqlx/struct.PgConnection.html)
```rust
pub struct PgConnection {
    pub(crate) stream: PgStream,
    process_id: u32,
    secret_key: u32,
    next_statement_id: Oid,
    cache_statement: StatementCache<(Oid, Arc<PgStatementMetadata>)>,
    cache_type_info: HashMap<Oid, PgTypeInfo>,
    cache_type_oid: HashMap<UStr, Oid>,
    pub(crate) pending_ready_for_query_count: usize,
    transaction_status: TransactionStatus,
    pub(crate) transaction_depth: usize,
    log_settings: LogSettings,
}

impl PgConnection {
    pub fn server_version_num(&self) -> Option<u32> {
        self.stream.server_version_num
    }

    pub(in crate::postgres) async fn wait_until_ready(&mut self) -> Result<(), Error> {
        if !self.stream.wbuf.is_empty() {
            self.stream.flush().await?;
        }

        while self.pending_ready_for_query_count > 0 {
            let message = self.stream.recv().await?;

            if let MessageFormat::ReadyForQuery = message.format {
                self.handle_ready_for_query(message)?;
            }
        }

        Ok(())
    }

    async fn recv_ready_for_query(&mut self) -> Result<(), Error> {
        let r: ReadyForQuery = self
            .stream
            .recv_expect(MessageFormat::ReadyForQuery)
            .await?;

        self.pending_ready_for_query_count -= 1;
        self.transaction_status = r.transaction_status;

        Ok(())
    }

    fn handle_ready_for_query(&mut self, message: Message) -> Result<(), Error> {
        self.pending_ready_for_query_count -= 1;
        self.transaction_status = ReadyForQuery::decode(message.contents)?.transaction_status;

        Ok(())
    }

    pub(crate) fn queue_simple_query(&mut self, query: &str) {
        self.pending_ready_for_query_count += 1;
        self.stream.write(Query(query));
    }
}

impl Debug for PgConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgConnection").finish()
    }
}

impl Connection for PgConnection {
    type Database = Postgres;

    type Options = PgConnectOptions;

    fn close(mut self) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async move {
            self.stream.send(Terminate).await?;
            self.stream.shutdown().await?;

            Ok(())
        })
    }

    fn close_hard(mut self) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async move {
            self.stream.shutdown().await?;

            Ok(())
        })
    }

    fn ping(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            self.write_sync();
            self.wait_until_ready().await
        })
    }

    fn begin(&mut self) -> BoxFuture<'_, Result<Transaction<'_, Self::Database>, Error>>
    where
        Self: Sized,
    {
        Transaction::begin(self)
    }

    fn cached_statements_size(&self) -> usize {
        self.cache_statement.len()
    }

    fn clear_cached_statements(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            let mut cleared = 0_usize;

            self.wait_until_ready().await?;

            while let Some((id, _)) = self.cache_statement.remove_lru() {
                self.stream.write(Close::Statement(id));
                cleared += 1;
            }

            if cleared > 0 {
                self.write_sync();
                self.stream.flush().await?;

                self.wait_for_close_complete(cleared).await?;
                self.recv_ready_for_query().await?;
            }

            Ok(())
        })
    }

    fn flush(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        self.wait_until_ready().boxed()
    }

    fn should_flush(&self) -> bool {
        !self.stream.wbuf.is_empty()
    }
}
```