use crate::AppState;
use crate::auth::AuthExtractor;
use crate::db::log as db_log;
use crate::model::{AnalysisRequest, AnalysisResults, Results, SqlResult};
use crate::runner::{RunResponse, RunnerInterface};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use futures::future::join_all;
use log::{error, warn};
use sea_orm::{ActiveModelTrait, NotSet, Set};
use std::sync::Arc;

#[utoipa::path(post, path = "/api/v1/analyse", request_body = AnalysisRequest, responses((status = OK, body = AnalysisResults), (status = UNAUTHORIZED), (status = BAD_REQUEST), (status = BAD_GATEWAY)), description = "Analyze SQL submission")]
pub async fn analyse(
    auth: AuthExtractor,
    state: State<AppState>,
    body: Json<AnalysisRequest>,
) -> Result<Json<AnalysisResults>, StatusCode> {
    let mut upstream_request = body.0.clone();
    if let Some(runner_interface) = &state.runner_interface {
        if upstream_request.solution_results.is_none() {
            upstream_request.solution_results = Some(
                generate_results(
                    &upstream_request.db_schema,
                    &upstream_request.solutions,
                    runner_interface,
                )
                .await,
            )
        }
        if upstream_request.submission_results.is_none() {
            upstream_request.submission_results = Some(
                generate_results(
                    &upstream_request.db_schema,
                    &upstream_request.submissions,
                    runner_interface,
                )
                .await,
            )
        }
    }

    let response = upstream_proxy(upstream_request, &state)
        .await
        .map_err(|e| {
            warn!("error from upstream: {}", e);
            StatusCode::BAD_GATEWAY
        })
        .map(Json)?;

    db_log::ActiveModel {
        id: NotSet,
        consumer_id: Set(auth.consumer_id),
        request: match serde_json::to_value(&body.0) {
            Ok(res) => Set(res),
            Err(_) => NotSet,
        },
        response: match serde_json::to_value(&response.0) {
            Ok(res) => Set(res),
            Err(_) => NotSet,
        },
    }
    .insert(&state.db)
    .await
    .map_err(|err| {
        error!("failed to store {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(response)
}

async fn generate_results(
    db_schema: &str,
    queries: &[String],
    runner_interface: &Arc<RunnerInterface>,
) -> Results {
    join_all(
        queries
            .iter()
            .map(|query| runner_interface.run(db_schema.to_string(), query.clone())),
    )
    .await
    .into_iter()
    .map(|r| {
        match r {
            Ok(i) => Some(i),
            Err(err) => {
                error!("error while contacting sql runner: {err}");
                None
            }
        }
        .map(|r| match r {
            RunResponse::Success(s) => SqlResult::Ok(s.result_set),
            RunResponse::Error(e) => SqlResult::Error(format!("Error: {}", e.error)),
        })
    })
    .collect()
}

async fn upstream_proxy(
    mut body: AnalysisRequest,
    state: &AppState,
) -> Result<AnalysisResults, anyhow::Error> {
    body.redact();
    let _permit = state.upstream_semaphore.acquire().await?;
    let res = reqwest::Client::new()
        .post(&state.config.upstream_url)
        .json(&body)
        .send()
        .await?;

    match res.error_for_status_ref() {
        Ok(_) => Ok(res.json().await?),
        Err(_) => Err(ProxyError::UpstreamError(res.status(), res.text().await?).into()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("unexpected code {0}: {1}")]
    UpstreamError(StatusCode, String),
}
