use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct Secret {
    pub id: Option<i32>,
    pub region: String,
    pub src: String,
    pub host: String,
    pub ip: String,
    pub port: i32,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
#[allow(dead_code)]
pub struct SyncList {
    pub id: Option<i32>,
    pub country: String,
    pub source_ip: String,
    pub source_port: String,
    pub source_username: String,
    pub source_password: String,
    pub dest_ip: String,
    pub dest_port: String,
    pub dest_username: String,
    pub dest_password: String,
    pub selected_tables: String, // JSON string
    pub creator: Option<String>,
    pub processor: Option<String>,
    pub remark: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub approval_status: Option<String>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct SyncSubmitRequest {
    pub country: String,
    pub source_ip: String,
    pub source_port: String,
    pub dest_ip: String,
    pub dest_port: String,
    pub selected_tables: serde_json::Value,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProxyWebhookRequest {
    pub country: String,
    pub ip: String,
    pub port: String,
    pub db: Option<String>,
    pub command: String,
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProcessorRequest {
    pub processor: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStatusRequest {
    pub approval_status: String,
}
