mod api;
mod auth;
#[allow(unused_imports)]
mod db;
mod model;
mod runner;

use crate::api::*;
use crate::runner::RunnerInterface;
use env_logger::Env;
use log::{LevelFilter, error, info};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use serde::Deserialize;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Semaphore;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_redoc::{Redoc, Servable};

fn get_default_port() -> u16 {
    8080
}

fn get_default_max_concurrent() -> usize {
    5
}

#[derive(Deserialize, Debug)]
struct Config {
    database_url: String,
    upstream_url: String,
    #[serde(default = "get_default_max_concurrent")]
    upstream_max_concurrent: usize,
    #[serde(default = "get_default_port")]
    port: u16,
    sql_runner_url: Option<String>,
}

#[derive(Debug, Clone)]
struct AppState {
    db: DatabaseConnection,
    upstream_semaphore: Arc<Semaphore>,
    runner_interface: Option<Arc<RunnerInterface>>,
    config: Arc<Config>,
}

#[derive(OpenApi)]
#[openapi(info(description = "API for analyzing SQL code submissions against solutions"))]
struct ApiDoc;

async fn run() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = envy::from_env::<Config>()?;

    let mut opt = ConnectOptions::new(&config.database_url);
    opt.sqlx_logging_level(LevelFilter::Debug);

    let db = Database::connect(opt).await?;

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(analyse))
        .split_for_parts();

    info!("Starting on port {}", config.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    axum::serve(
        listener,
        router
            .merge(Redoc::with_url("/redoc", api))
            .with_state(AppState {
                db,
                upstream_semaphore: Arc::new(Semaphore::new(config.upstream_max_concurrent)),
                runner_interface: config.sql_runner_url.as_ref().map(|url| {
                    Arc::new(RunnerInterface::new(
                        url.parse().expect("failed to parse SQL_RUNNER_URL"),
                    ))
                }),
                config: Arc::new(config),
            }),
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
