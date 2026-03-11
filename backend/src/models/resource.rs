use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, utoipa::ToSchema)]
pub struct ResourceDataSource {
    pub id: i32,
    pub name: String,
    pub r#type: String,
    pub url: String,
    pub username: Option<String>,
    #[serde(skip_serializing)] 
    pub password: Option<String>,
    pub region: Option<String>,
    pub fe_mapping: Option<Value>,
    pub connection_timeout: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, utoipa::ToSchema)]
pub struct ResourcePanel {
    pub id: i32,
    pub section: String, 
    pub title: String,
    pub chart_type: String, 
    pub promql_query: String,
    pub config: Option<Value>,
    pub display_order: i32,
    pub data_source_id: Option<i32>,
    pub country: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct CreatePanelRequest {
    pub section: Option<String>,
    pub title: String,
    pub chart_type: String,
    pub promql_query: String,
    pub config: Option<Value>,
    pub display_order: Option<i32>,
    pub data_source_id: Option<i32>,
    pub country: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct UpdatePanelRequest {
    pub title: Option<String>,
    pub chart_type: Option<String>,
    pub promql_query: Option<String>,
    pub config: Option<Value>,
    pub display_order: Option<i32>,
    pub data_source_id: Option<i32>,
    pub country: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct CreateDataSourceRequest {
    pub name: String,
    pub r#type: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub region: Option<String>,
    pub fe_mapping: Option<Value>,
    pub connection_timeout: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct UpdateDataSourceRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub region: Option<String>,
    pub fe_mapping: Option<Value>,
    pub connection_timeout: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct TestDataSourceRequest {
    pub id: Option<i32>,
    pub r#type: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub fe_mapping: Option<Value>,
    pub connection_timeout: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::IntoParams)]
pub struct PromQuery {
    pub data_source_id: Option<i32>,
    pub query: String,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub step: Option<String>,
}
