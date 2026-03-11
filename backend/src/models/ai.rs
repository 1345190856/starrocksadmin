use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema, Clone)]
pub struct AiSetting {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub body: Option<String>,
    pub category: String,
    pub is_published: i8, // Use i8 for TINYINT(1) to be strictly 0/1
    pub creator: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAiSettingRequest {
    pub name: String,
    pub url: String,
    pub body: Option<String>,
    pub category: String,
    pub is_published: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAiSettingRequest {
    pub name: String,
    pub url: String,
    pub body: Option<String>,
    pub category: String,
    pub is_published: bool,
}
