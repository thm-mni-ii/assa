mod routes;

use env_logger::Env;
use log::{error, info};
use serde::Deserialize;
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

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "get_default_port")]
    port: u16,
    base_url: String,
    openai_api_key: String,
    model: String,
}

#[derive(OpenApi)]
#[openapi(info(description = "API for generating feedback using llms"))]
struct ApiDoc;

async fn run() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = envy::from_env::<Config>()?;

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(routes::generate_feedback))
        .split_for_parts();

    info!("Starting on port {}", config.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    axum::serve(
        listener,
        router
            .merge(Redoc::with_url("/redoc", api))
            .with_state(Arc::new(config)),
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
