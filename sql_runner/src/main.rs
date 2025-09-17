mod db;
mod routes;

use crate::db::DB;
use env_logger::Env;
use log::{error, info};
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer};
use std::process::exit;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_redoc::Redoc;
use utoipa_redoc::Servable;

fn get_default_port() -> u16 {
    8080
}

fn get_default_max_rows_in_result_set() -> usize {
    1000
}

fn get_default_statement_timeout() -> u64 {
    10000
}

pub fn hex_to_bytes32<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let bytes = hex::decode(s).map_err(SerdeError::custom)?;
    if bytes.len() != 32 {
        return Err(SerdeError::custom(format!(
            "Expected 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "get_default_port")]
    port: u16,
    db_password: String,
    db_username: String,
    db_host: String,
    #[serde(deserialize_with = "hex_to_bytes32")]
    password_hash_key: [u8; 32],
    #[serde(default = "get_default_max_rows_in_result_set")]
    max_rows_in_result_set: usize,
    #[serde(default = "get_default_statement_timeout")]
    statement_timeout: u64,
}

#[derive(Debug, Clone)]
struct AppState {
    db: Arc<DB>,
}

#[derive(OpenApi)]
#[openapi(info(description = "API for comparing result sets"))]
struct ApiDoc;

async fn run() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = envy::from_env::<Config>()?;

    let db = Arc::new(
        DB::connect(
            config.db_host.clone(),
            config.db_username.clone(),
            config.db_password.clone(),
            config.password_hash_key,
            config.max_rows_in_result_set,
            config.statement_timeout,
        )
        .await?,
    );

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(routes::run))
        .routes(routes!(routes::compare_result_set))
        .routes(routes!(routes::batch_compare_result_sets))
        .split_for_parts();

    info!("Starting on port {}", config.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    axum::serve(
        listener,
        router
            .merge(Redoc::with_url("/redoc", api))
            .with_state(AppState { db }),
    )
    .await?;

    Ok(())
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    if let Err(err) = rt.block_on(run()) {
        error!("{}", err);
        exit(1)
    }
}
