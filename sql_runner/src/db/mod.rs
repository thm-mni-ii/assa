mod introspect;
pub mod types;

use crate::db::types::{DatabaseInfo, ResultSet, ResultSetExtension, SqlValue};
use futures::{StreamExt, TryStreamExt};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::types::Decimal;
use sqlx::{Column, Executor, FromRow, Pool, Postgres, Row};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use utoipa::ToSchema;

type DatabaseType = Postgres;
type RowType = PgRow;

#[derive(Debug)]
pub struct DB {
    root_connection: Pool<DatabaseType>,
    connections: Mutex<HashMap<String, Arc<Pool<DatabaseType>>>>,
    password_hash_key: [u8; 32],
    db_host: String,
    db_root_username: String,
    db_root_password: String,
    max_rows_in_result_set: usize,
    statement_timeout: u64,
    create_db_mutex: Mutex<()>,
}

impl DB {
    pub async fn connect(
        db_host: String,
        db_root_username: String,
        db_root_password: String,
        password_hash_key: [u8; 32],
        max_rows_in_result_set: usize,
        statement_timeout: u64,
    ) -> Result<Self, SqlExecutionError> {
        Ok(DB {
            root_connection: PgPoolOptions::new()
                .connect(&format!(
                    "postgresql://{}:{}@{}",
                    db_root_username, db_root_password, db_host
                ))
                .await?,
            connections: Default::default(),
            password_hash_key,
            db_host,
            db_root_username,
            db_root_password,
            max_rows_in_result_set,
            statement_timeout,
            create_db_mutex: Default::default(),
        })
    }

    pub async fn execute(
        &self,
        environment: &str,
        query: &str,
        include_database_info: bool,
    ) -> Result<(ResultSet, Option<DatabaseInfo>), SqlExecutionError> {
        let environment_hash = blake3::hash(environment.as_bytes()).to_hex().to_string();
        let db_name = &environment_hash[..63];
        let password_hash =
            blake3::keyed_hash(&self.password_hash_key, environment_hash.as_bytes())
                .to_hex()
                .to_string();
        let db_exists = self.db_exists(db_name).await?;

        let conn = if !db_exists {
            self.create_db(environment, db_name, &password_hash).await?
        } else {
            self.get_connection(db_name, db_name, &password_hash)
                .await?
        };

        debug!("Executing query in {db_name}");
        let result_set = self.extract(&*conn, query).await?;
        let database_info = if include_database_info {
            Some(self.get_database_information(&*conn).await?)
        } else {
            None
        };
        Ok((result_set, database_info))
    }

    async fn db_exists(&self, db_name: &str) -> Result<bool, SqlExecutionError> {
        Ok(sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
            .bind(db_name)
            .fetch_optional(&self.root_connection)
            .await?
            .is_some())
    }

    async fn create_db(
        &self,
        environment: &str,
        db_name: &str,
        password_hash: &str,
    ) -> Result<Arc<Pool<DatabaseType>>, SqlExecutionError> {
        let _create_db_lock = self.create_db_mutex.lock().await;
        let db_exists = self.db_exists(db_name).await?;

        if !db_exists {
            debug!("Creating database {db_name}");
            self.create_database_and_user(db_name, password_hash)
                .await?;
        }

        let conn = self.get_connection(db_name, db_name, password_hash).await?;

        if !db_exists {
            debug!("Initialising database {db_name}");
            self.init_environment(&*conn, environment).await?;
            debug!("Updating permission for database {db_name}");
            let root_conn = self
                .get_connection(db_name, &self.db_root_username, &self.db_root_password)
                .await?;
            self.make_database_readonly(&*root_conn, db_name).await?;
        }

        Ok(conn)
    }

    pub async fn compare(
        &self,
        environment: &str,
        query_a: &str,
        query_b: &str,
        row_norm: RowNormalisation,
        col_norm: ColumnNormalisation,
    ) -> Result<(ResultSet, ResultSet, bool), SqlExecutionError> {
        let (mut result_a, _) = self.execute(environment, query_a, false).await?;
        let (mut result_b, _) = self.execute(environment, query_b, false).await?;

        if col_norm == ColumnNormalisation::NumberColumnsByOrder {
            result_a.number_columns();
            result_b.number_columns();
        } else if col_norm == ColumnNormalisation::SortColumnsByName {
            result_a.sort_columns();
            result_b.sort_columns();
        }
        if row_norm == RowNormalisation::SortRows {
            result_a.sort_rows();
            result_b.sort_rows();
        }

        let eq = result_a == result_b;
        Ok((result_a, result_b, eq))
    }

    // Name and password must be trusted as queries used to create database
    async fn create_database_and_user(
        &self,
        name: &str,
        password: &str,
    ) -> Result<(), SqlExecutionError> {
        self.root_connection
            .execute(format!("CREATE DATABASE \"{name}\";").as_str())
            .await?;
        self.root_connection
            .execute(
                format!("CREATE USER \"{name}\" WITH ENCRYPTED PASSWORD '{password}';").as_str(),
            )
            .await?;
        self.root_connection
            .execute(format!("ALTER DATABASE \"{name}\" OWNER TO \"{name}\";").as_str())
            .await?;

        Ok(())
    }

    // Name must be trusted as queries used to change permission don't support bind
    async fn make_database_readonly<'c, E: Executor<'c, Database = DatabaseType> + Copy>(
        &self,
        root_conn: E,
        name: &str,
    ) -> Result<(), SqlExecutionError> {
        root_conn
            .execute(
                format!(
                    "REASSIGN OWNED BY \"{name}\" TO \"{}\";",
                    self.db_root_username
                )
                .as_str(),
            )
            .await?;
        root_conn
            .execute(
                format!(
                    "ALTER DATABASE \"{name}\" OWNER TO \"{}\";",
                    self.db_root_username
                )
                .as_str(),
            )
            .await?;
        root_conn
            .execute(format!("GRANT CONNECT ON DATABASE \"{name}\" TO \"{name}\";").as_str())
            .await?;
        root_conn
            .execute(format!("GRANT USAGE ON SCHEMA public TO \"{name}\";").as_str())
            .await?;
        root_conn
            .execute(format!("GRANT SELECT ON ALL TABLES IN SCHEMA public TO \"{name}\";").as_str())
            .await?;
        Ok(())
    }

    async fn get_connection(
        &self,
        db: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<Arc<Pool<DatabaseType>>, SqlExecutionError> {
        let mut connections = self.connections.lock().await;
        let mut connection_option = connections.get(&format!("{username}@{db}"));
        let connection = match connection_option {
            None => {
                let pool = PgPoolOptions::new()
                    .max_connections(1)
                    .connect(&format!(
                        "postgresql://{}:{}@{}/{}",
                        username, password_hash, self.db_host, db
                    ))
                    .await?;
                pool.execute(
                    format!("SET statement_timeout to {}", self.statement_timeout).as_str(),
                )
                .await?;
                connections.insert(db.to_string(), Arc::new(pool));
                connection_option = connections.get(db);
                connection_option.unwrap()
            }
            Some(option) => option,
        };
        Ok(connection.clone())
    }

    async fn extract<'c, E: Executor<'c, Database = DatabaseType>>(
        &self,
        conn: E,
        query: &str,
    ) -> Result<ResultSet, SqlExecutionError> {
        let rows = sqlx::query(query)
            .fetch(conn)
            .take(self.max_rows_in_result_set)
            .try_collect::<Vec<PgRow>>()
            .await
            .map_err(SqlExecutionError::Execute)?;
        let mut cell: OnceCell<ResultSet> = OnceCell::new();
        let row_len = rows.len();
        for row in rows {
            cell.get_or_init(|| ResultSet {
                columns: row
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect(),
                rows: Vec::with_capacity(row_len),
            });
            let cell_ref = cell.get_mut().unwrap();
            let mut row_set = Vec::with_capacity(row.columns().len());
            for column in row.columns() {
                if let Ok(str) = row.try_get::<String, _>(column.name()) {
                    row_set.push(SqlValue::Text(str))
                } else if let Ok(d) = row.try_get::<Decimal, _>(column.name()) {
                    row_set.push(SqlValue::Float(d.try_into().map_err(|_| {
                        SqlExecutionError::ColumnDecodeError(column.name().to_string())
                    })?))
                } else if let Ok(f) = row.try_get::<f64, _>(column.name()) {
                    row_set.push(SqlValue::Float(f))
                } else if let Ok(f) = row.try_get::<f32, _>(column.name()) {
                    row_set.push(SqlValue::Float(f.into()))
                } else if let Ok(i) = row.try_get::<i64, _>(column.name()) {
                    row_set.push(SqlValue::Int(i))
                } else if let Ok(i) = row.try_get::<i32, _>(column.name()) {
                    row_set.push(SqlValue::Int(i.into()))
                } else if let Ok(b) = row.try_get::<bool, _>(column.name()) {
                    row_set.push(SqlValue::Bool(b))
                } else if let Ok(c) = row.try_get::<sqlx::types::chrono::NaiveDateTime, _>(column.name()) {
                    row_set.push(SqlValue::Text(c.to_string()))
                } else if let Ok(c) = row.try_get::<sqlx::types::chrono::NaiveDate, _>(column.name()) {
                    row_set.push(SqlValue::Text(c.to_string()))
                } else {
                    return Err(SqlExecutionError::ColumnDecodeError(
                        column.name().to_string(),
                    ));
                }
            }
            cell_ref.rows.push(row_set);
        }

        Ok(cell.into_inner().unwrap_or_else(|| ResultSet {
            columns: vec![],
            rows: vec![],
        }))
    }

    async fn init_environment<'c, E: Executor<'c, Database = DatabaseType>>(
        &self,
        conn: E,
        environment: &str,
    ) -> Result<(), SqlExecutionError> {
        let mut results = conn.execute_many(environment);
        while let Some(r) = results.next().await {
            if let Err(err) = r {
                return Err(SqlExecutionError::Init(err));
            }
        }
        Ok(())
    }

    async fn get_database_information<'c, E: Executor<'c, Database = DatabaseType> + Copy>(
        &self,
        conn: E,
    ) -> Result<DatabaseInfo, SqlExecutionError> {
        Ok(DatabaseInfo {
            tables: self.introspect(conn, introspect::TABLES).await?,
            constraints: self.introspect(conn, introspect::CONSTRAINTS).await?,
            views: self.introspect(conn, introspect::VIEWS).await?,
            routines: self.introspect(conn, introspect::ROUTINES).await?,
            triggers: self.introspect(conn, introspect::TRIGGERS).await?,
        })
    }

    async fn introspect<
        'c,
        E: Executor<'c, Database = DatabaseType>,
        T: for<'r> FromRow<'r, RowType> + Send + Unpin,
    >(
        &self,
        conn: E,
        query: &str,
    ) -> Result<Vec<T>, SqlExecutionError> {
        Ok(sqlx::query_as(query).fetch_all(conn).await?)
    }
}

#[derive(Error, Debug)]
pub enum SqlExecutionError {
    #[error("error while initializing database: {0}")]
    Init(sqlx::Error),
    #[error("error while executing supplied query: {0}")]
    Execute(sqlx::Error),
    #[error("an sql error occurred: {0}")]
    Other(#[from] sqlx::Error),
    #[error("failed to determine column type of `{0}`")]
    ColumnDecodeError(String),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, ToSchema)]
pub enum RowNormalisation {
    NoNormalization,
    SortRows,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, ToSchema)]
pub enum ColumnNormalisation {
    NoNormalization,
    SortColumnsByName,
    NumberColumnsByOrder,
}
