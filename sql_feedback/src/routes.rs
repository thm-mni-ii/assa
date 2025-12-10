use crate::Config;
use askama::Template;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use common::models::Results;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Template)]
#[template(path = "prompt.txt")]
struct PromptTemplate<'a> {
    request: &'a FeedbackRequest,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FeedbackRequest {
    pub sql_environment: String,
    pub db_schema: String,
    pub task: String,
    pub solutions: Vec<String>,
    pub submissions: Vec<String>,
    pub solution_results: Option<Results>,
    pub submission_results: Option<Results>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeedbackResponse {
    pub correct: bool,
    pub feedback: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeedbackErrorResponse {
    pub code: u16,
    pub message: &'static str,
}

#[utoipa::path(post, path = "/api/v1/feedback", request_body = FeedbackRequest, responses((status = OK, body = FeedbackResponse), (status = UNPROCESSABLE_ENTITY), (status = INTERNAL_SERVER_ERROR)), description = "Gets feedback")]
#[axum::debug_handler]
pub async fn generate_feedback(
    config: State<Arc<Config>>,
    body: Json<FeedbackRequest>,
) -> Result<Json<Vec<FeedbackResponse>>, (StatusCode, Json<FeedbackErrorResponse>)> {
    let prompt = PromptTemplate { request: &body.0 }.render().unwrap();

    let response = reqwest::Client::new()
        .post(format!("{}/chat/completions", config.base_url))
        .bearer_auth(&config.openai_api_key)
        .json(&json!({
            "model": config.model,
            "messages": vec![json!({"role": "user", "content": prompt})],
            "temperature": 0,
        }))
        .send()
        .await
        .and_then(|response| response.error_for_status());

    let response = match response {
        Ok(response) => response,
        Err(e) => {
            error!("error while sending llm request: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FeedbackErrorResponse {
                    code: 500,
                    message: "an error occurred while sending llm request",
                }),
            ));
        }
    };

    let body = match response.json::<serde_json::Value>().await {
        Ok(body) => body,
        Err(e) => {
            error!("error while parsing llm response: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FeedbackErrorResponse {
                    code: 500,
                    message: "an error occurred while parsing the llm response",
                }),
            ));
        }
    };
    let message = body["choices"][0]["message"]["content"].as_str();

    let message = match message {
        Some(message) => message,
        None => {
            error!("error while processing llm response: choices[0].message.content not found");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FeedbackErrorResponse {
                    code: 500,
                    message: "an error occurred while processing the llm response",
                }),
            ));
        }
    };

    Ok(Json(vec![FeedbackResponse {
        correct: false,
        feedback: message.to_string(),
    }]))
}
