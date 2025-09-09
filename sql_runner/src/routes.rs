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
pub struct GenerateRequest {
    pub environment: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GenerateResponse {
    pub result_set: ResultSet,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GenerateError {
    pub location: &'static str,
    pub error: String,
}

type GenerateErrorResponse = (StatusCode, Json<GenerateError>);

#[utoipa::path(post, path = "/api/v1/run", request_body = GenerateRequest, responses((status = OK, body = GenerateResponse), (status = UNAUTHORIZED), (status = BAD_REQUEST)), description = "Analyze SQL submission")]
#[axum::debug_handler]
pub async fn run(
    state: State<AppState>,
    body: Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, GenerateErrorResponse> {
    let (rs, _) = state
        .db
        .execute(&body.environment, &body.query, false)
        .await
        .map_err(|err| {
            error!("Error while handling run request: {err}");
            err_to_response(err)
        })?;
    Ok(Json(GenerateResponse { result_set: rs }))
}

fn err_to_response(err: SqlExecutionError) -> (StatusCode, Json<GenerateError>) {
    match err {
        SqlExecutionError::Init(e) => (
            StatusCode::BAD_REQUEST,
            Json(GenerateError {
                location: "init",
                error: e.to_string(),
            }),
        ),
        SqlExecutionError::Execute(e) => (
            StatusCode::BAD_REQUEST,
            Json(GenerateError {
                location: "query",
                error: e.to_string(),
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenerateError {
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
    pub solution: GenerateResponse,
    pub submission: GenerateResponse,
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
        solution: GenerateResponse { result_set: a },
        submission: GenerateResponse { result_set: b },
        equal: eq,
    }))
}
