use crate::AppState;
use crate::db::types::ResultSet;
use crate::db::{CompareMode, SqlExecutionError};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use log::error;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RunRequest {
    pub environment: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RunResponse {
    pub result_set: ResultSet,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RunError {
    pub location: &'static str,
    pub error: String,
}

type GenerateErrorResponse = (StatusCode, Json<RunError>);

#[utoipa::path(post, path = "/api/v1/run", request_body = RunRequest, responses((status = OK, body = RunResponse), (status = UNAUTHORIZED), (status = BAD_REQUEST)), description = "Analyze SQL submission")]
#[axum::debug_handler]
pub async fn run(
    state: State<AppState>,
    body: Json<RunRequest>,
) -> Result<Json<RunResponse>, GenerateErrorResponse> {
    let (rs, _) = state
        .db
        .execute(&body.environment, &body.query, false)
        .await
        .map_err(|err| {
            error!("Error while handling run request: {err}");
            err_to_response(err)
        })?;
    Ok(Json(RunResponse { result_set: rs }))
}

fn err_to_response(err: SqlExecutionError) -> (StatusCode, Json<RunError>) {
    match err {
        SqlExecutionError::Init(e) => (
            StatusCode::OK,
            Json(RunError {
                location: "init",
                error: e.to_string(),
            }),
        ),
        SqlExecutionError::Execute(e) => (
            StatusCode::OK,
            Json(RunError {
                location: "query",
                error: e.to_string(),
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RunError {
                location: "other",
                error: "an internal error occurred".to_string(),
            }),
        ),
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CompareRequest {
    pub environment: String,
    pub solution: String,
    pub submission: String,
    pub mode: CompareMode,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CompareResponse {
    pub solution: RunResponse,
    pub submission: RunResponse,
    pub equal: bool,
}

#[utoipa::path(post, path = "/api/v1/compare", request_body = CompareRequest, responses((status = OK, body = CompareResponse), (status = UNAUTHORIZED), (status = BAD_REQUEST), (status = BAD_GATEWAY)), description = "Analyze SQL submission")]
#[axum::debug_handler]
pub async fn compare_result_set(
    state: State<AppState>,
    body: Json<CompareRequest>,
) -> Result<Json<CompareResponse>, GenerateErrorResponse> {
    let (a, b, eq) = state
        .db
        .compare(
            &body.environment,
            &body.solution,
            &body.submission,
            body.mode,
        )
        .await
        .map_err(|err| {
            error!("Error while handling compare_result_set request: {err}");
            err_to_response(err)
        })?;
    Ok(Json(CompareResponse {
        solution: RunResponse { result_set: a },
        submission: RunResponse { result_set: b },
        equal: eq,
    }))
}
