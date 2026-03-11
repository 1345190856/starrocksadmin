use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct ResourceAsset {
    pub instance_type: Option<String>,
    pub instance_name: Option<String>,
    pub instance_id: Option<String>,
    pub private_ip: String,
    pub public_ip: Option<String>,
    pub manual_service: Option<String>,
    pub auto_services: Option<serde_json::Value>,
    pub status: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
    pub network_identifier: Option<String>,
    pub release: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub project_name: Option<String>,
    pub project_ownership: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct ResourceQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub query: Option<String>,
    pub instance_type: Option<String>,
    pub project_name: Option<String>,
    pub manual_service: Option<String>,
    pub country: Option<String>,
    pub status: Option<String>,
    pub region: Option<String>,
    pub service_status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResourceListResponse {
    pub list: Vec<ResourceAsset>,
    pub total: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResourceImportRequest {
    pub items: Vec<ResourceAssetImport>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct ResourceAssetImport {
    pub instance_type: Option<String>,
    pub instance_name: Option<String>,
    pub instance_id: Option<String>,
    pub private_ip: String,
    pub public_ip: Option<String>,
    pub manual_service: Option<String>,
    pub auto_services: Option<serde_json::Value>,
    pub status: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
    pub network_identifier: Option<String>,
    pub release: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub project_name: Option<String>,
    pub project_ownership: Option<String>,
    pub created_at: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResourceFilterOptions {
    pub project_names: Vec<String>,
    pub service_types: Vec<String>,
    pub service_statuses: Vec<String>,
    pub countries: Vec<String>,
    pub regions: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResourceBatchDeleteRequest {
    pub private_ips: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResourceApplyRequest {
    pub ip_list: Vec<String>,
    pub cookie: String,
    pub remarks: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResourceApplyResponse {
    pub total_count: usize,
    pub success_count: usize,
    pub failed_ips: Vec<String>,
    pub not_found_ips: Vec<String>,
    pub error_msg: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResourceServiceOpRequest {
    #[serde(rename = "type")]
    pub op_type: String,
    pub service: String,
    pub ip: String,
}
