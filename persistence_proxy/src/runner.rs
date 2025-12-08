pub use common::models::ResultSet;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct RunnerInterface {
    client: Client,
    run_url: Url,
}

impl RunnerInterface {
    pub fn new(run_url: Url) -> Self {
        RunnerInterface {
            client: Client::new(),
            run_url,
        }
    }

    pub async fn run(
        &self,
        environment: String,
        query: String,
    ) -> Result<RunResponse, anyhow::Error> {
        Ok(self
            .client
            .post(self.run_url.clone())
            .json(&RunRequest { environment, query })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRequest {
    pub environment: String,
    pub query: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunSuccessResponse {
    pub result_set: ResultSet,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RunSuccessErrorResponse {
    pub location: String,
    pub error: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RunResponse {
    Success(RunSuccessResponse),
    Error(RunSuccessErrorResponse),
}
