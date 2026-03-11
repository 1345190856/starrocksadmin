use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, utoipa::ToSchema)]
pub struct Application {
    pub id: i32,
    pub name: String,
    pub r#type: String, // prometheus, mysql, grafana
    pub address: String,
    pub region: String, // China, Pakistan, Indonesia, Philippines, Thailand, Mexico
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub r#type: String,
    pub address: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct UpdateApplicationRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub address: Option<String>,
    pub region: Option<String>,
}
