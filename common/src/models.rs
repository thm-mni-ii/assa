use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, PartialOrd)]
#[serde(untagged)]
pub enum SqlValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, PartialOrd)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, PartialOrd)]
pub enum SqlResult {
    Ok(ResultSet),
    Error(String),
}

pub type Results = Vec<Option<SqlResult>>;
