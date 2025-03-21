use crate::AppState;
use crate::auth::AuthExtractor;
use crate::db::log as db_log;
use crate::model::{AnalysisRequest, AnalysisResults};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use log::{error, warn};
use sea_orm::{ActiveModelTrait, NotSet, Set};
use std::sync::LazyLock;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[utoipa::path(post, path = "/api/v1/analyse", request_body = AnalysisRequest, responses((status = OK, body = AnalysisResults), (status = UNAUTHORIZED), (status = BAD_REQUEST), (status = BAD_GATEWAY)), description = "Analyze SQL submission")]
pub async fn analyse(
    auth: AuthExtractor,
    state: State<AppState>,
    body: Json<AnalysisRequest>,
) -> Result<Json<AnalysisResults>, StatusCode> {
    let response = upstream_proxy(body.0.clone(), &state)
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
        response: match serde_json::to_value(&body.0) {
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

async fn upstream_proxy(
    mut body: AnalysisRequest,
    state: &AppState,
) -> Result<AnalysisResults, anyhow::Error> {
    body.user_id.take();
    let _permit = state.upstream_semaphore.acquire().await?;
    let res = HTTP_CLIENT
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
