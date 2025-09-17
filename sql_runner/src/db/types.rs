use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, PartialOrd)]
#[serde(untagged)]
pub enum SqlValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, PartialOrd)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlValue>>,
}

impl ResultSet {
    pub fn sort_columns(&mut self) {
        let mut indexed_columns = self.columns.iter().enumerate().collect::<Vec<_>>();
        indexed_columns.sort_by(|(_, column_a), (_, column_b)| column_a.cmp(column_b));
        let new_rows = self
            .rows
            .iter()
            .map(|row| {
                let mut new_row = row.clone();
                for (new_index, (old_index, _)) in indexed_columns.iter().enumerate() {
                    new_row[new_index] = row[*old_index].clone();
                }
                new_row
            })
            .collect();
        let new_columns = indexed_columns
            .into_iter()
            .map(|(_, col_b)| col_b.to_string())
            .collect();
        self.rows = new_rows;
        self.columns = new_columns;
    }

    pub fn number_columns(&mut self) {
        self.columns = (0..self.columns.len()).map(|i| i.to_string()).collect();
    }

    pub fn sort_rows(&mut self) {
        self.rows.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DatabaseInfo {
    pub tables: Vec<TableDatabaseInfo>,
    pub constraints: Vec<ConstraintsDatabaseInfo>,
    pub views: Vec<ViewDatabaseInfo>,
    pub routines: Vec<RoutineDatabaseInfo>,
    pub triggers: Vec<TriggerDatabaseInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TableColumnInfo {
    pub name: String,
    #[serde(rename = "isNullable")]
    pub is_nullable: bool,
    #[serde(rename = "udtName")]
    pub udt_name: String,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TableDatabaseInfo {
    pub name: String,
    #[sqlx(json)]
    pub json: Vec<TableColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConstraintInfo {
    #[serde(rename = "columnName")]
    pub column_name: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "check_clause")]
    pub check_clause: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ConstraintsDatabaseInfo {
    #[serde(rename = "table")]
    pub table_name: String,
    #[sqlx(json)]
    pub json: Vec<ConstraintInfo>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ViewDatabaseInfo {
    #[serde(rename = "table")]
    pub table_name: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct RoutineDatabaseInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub definition: Option<String>,
    pub parameters: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TriggerDatabaseInfo {
    pub name: String,
    #[serde(rename = "objectTable")]
    pub object_table: String,
    pub json: Vec<String>,
    pub statement: String,
    pub orientation: String,
    pub timing: String,
}
