use crate::AppState;
use crate::db::types::ResultSet;
use crate::db::{CompareMode, SqlExecutionError};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use futures::future::join_all;
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

#[utoipa::path(post, path = "/api/v1/run", request_body = RunRequest, responses((status = OK, body = RunResponse), (status = UNPROCESSABLE_ENTITY), (status = INTERNAL_SERVER_ERROR)), description = "Execute query in environment")]
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

fn err_to_response(err: SqlExecutionError) -> GenerateErrorResponse {
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
        e => {
            error!("internal error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RunError {
                    location: "other",
                    error: "an internal error occurred".to_string(),
                }),
            )
        }
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

#[utoipa::path(post, path = "/api/v1/compare", request_body = CompareRequest, responses((status = OK, body = CompareResponse), (status = UNPROCESSABLE_ENTITY), (status = INTERNAL_SERVER_ERROR)), description = "Compare sql result sets")]
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchCompareRequest {
    pub environment: String,
    pub solutions: Vec<(String, CompareMode)>,
    pub submission: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchCompareResponse {
    pub equal: Vec<bool>,
}

#[utoipa::path(post, path = "/api/v1/batch_compare", request_body = BatchCompareRequest, responses((status = OK, body = BatchCompareResponse), (status = UNPROCESSABLE_ENTITY), (status = INTERNAL_SERVER_ERROR)), description = "Batch compare SQL resulsets")]
pub async fn batch_compare_result_sets(
    state: State<AppState>,
    body: Json<BatchCompareRequest>,
) -> Result<Json<BatchCompareResponse>, GenerateErrorResponse> {
    let eq = join_all(body.solutions.iter().map(|(solution_ref, mode)| async {
        state
            .db
            .compare(&body.environment, solution_ref, &body.submission, *mode)
            .await
            .map_err(|err| {
                error!("Error while handling compare_result_set request: {err}");
                err_to_response(err)
            })
            .map(|(_, _, eq)| eq)
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<bool>, GenerateErrorResponse>>()?;

    Ok(Json(BatchCompareResponse { equal: eq }))
}
