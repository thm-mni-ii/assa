pub use common::models::{Results, SqlResult};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AnalysisRequest {
    pub sql_environment: String,
    pub db_schema: String,
    pub task: String,
    pub solutions: Vec<String>,
    pub submissions: Vec<String>,
    pub solution_results: Option<Results>,
    pub submission_results: Option<Results>,
    pub task_id: Option<String>,
    pub user_id: Option<String>,
    pub feedback_language: Option<String>,
}

impl AnalysisRequest {
    pub fn redact(&mut self) {
        self.task_id.take();
        self.user_id.take();
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AnalysisResult {
    pub correct: bool,
    pub feedback: String,
}

pub type AnalysisResults = Vec<AnalysisResult>;
