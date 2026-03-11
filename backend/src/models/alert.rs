use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum AlertRegion {
    China,
    Thailand,
    Mexico,
    Philippines,
    Pakistan,
    Indonesia,
    #[serde(other)]
    Unknown,
}

impl AsRef<str> for AlertRegion {
    fn as_ref(&self) -> &str {
        match self {
            Self::China => "China",
            Self::Thailand => "Thailand",
            Self::Mexico => "Mexico",
            Self::Philippines => "Philippines",
            Self::Pakistan => "Pakistan",
            Self::Indonesia => "Indonesia",
            Self::Unknown => "Unknown",
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum AlertSubType {
    Memory,
    Cpu,
    ScanRows,
    ExecutionTime,
    #[serde(other)]
    Unknown,
}

impl AsRef<str> for AlertSubType {
    fn as_ref(&self) -> &str {
        match self {
            Self::Memory => "Memory",
            Self::Cpu => "Cpu",
            Self::ScanRows => "ScanRows",
            Self::ExecutionTime => "ExecutionTime",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertReceiver {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: String, // "duty" or "manager"
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannel {
    pub r#type: String, // "tv" or "ivr"
    pub start_time: String, // "00:00"
    pub end_time: String, // "24:00"
    // TV Config
    pub template_id: Option<String>,
    // IVR Config
    pub ivr_template: Option<String>,
    pub ivr_secret: Option<String>,
    pub ivr_params: Option<serde_json::Value>,
    pub notify_interval_minutes: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub id: i32,
    pub name: String,
    pub region: String, // stored as string in DB
    pub tags: Option<String>,
    pub data_source: String,
    pub datasource_id: Option<i32>,
    pub alert_type: String, // default "Abnormal SQL"
    pub sub_type: String,
    pub threshold: i64,
    pub starrocks_version: String,
    pub channel: Option<String>, // "tv" or "ivr", default "tv"
    pub template_id: Option<String>, // For TV (bot_id)
    pub ivr_template: Option<String>, // For IVR
    pub ivr_secret: Option<String>, // For IVR
    #[sqlx(json)]
    pub ivr_params: Option<serde_json::Value>, // For IVR
    #[sqlx(json)]
    pub channels: Option<Vec<AlertChannel>>, // Multiple channels
    #[sqlx(json)]
    pub receivers: Vec<AlertReceiver>,
    pub enabled: bool,
    pub auto_kill: bool,
    pub auto_kill_threshold_minutes: Option<i32>,
    pub notify_interval_minutes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub region: String,
    pub tags: Option<String>,
    pub data_source: String,
    pub datasource_id: Option<i32>,
    pub alert_type: Option<String>,
    pub sub_type: String,
    pub threshold: i64,
    pub starrocks_version: String,
    pub channel: Option<String>,
    pub template_id: Option<String>,
    pub ivr_template: Option<String>,
    pub ivr_secret: Option<String>,
    pub ivr_params: Option<serde_json::Value>,
    pub channels: Option<Vec<AlertChannel>>,
    pub receivers: Vec<AlertReceiver>,
    pub enabled: bool,
    pub auto_kill: Option<bool>,
    pub auto_kill_threshold_minutes: Option<i32>,
    pub notify_interval_minutes: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub region: Option<String>,
    pub tags: Option<String>,
    pub data_source: Option<String>,
    pub datasource_id: Option<i32>,
    pub sub_type: Option<String>,
    pub threshold: Option<i64>,
    pub starrocks_version: Option<String>,
    pub channel: Option<String>,
    pub template_id: Option<String>,
    pub ivr_template: Option<String>,
    pub ivr_secret: Option<String>,
    pub ivr_params: Option<serde_json::Value>,
    pub channels: Option<Vec<AlertChannel>>,
    pub receivers: Option<Vec<AlertReceiver>>,
    pub enabled: Option<bool>,
    pub auto_kill: Option<bool>,
    pub auto_kill_threshold_minutes: Option<i32>,
    pub notify_interval_minutes: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertHistory {
    pub id: i32,
    pub rule_id: i32,
    pub query_id: String,
    pub start_time: Option<String>,
    pub user: Option<String>,
    pub host: Option<String>,
    pub db: Option<String>,
    pub department: Option<String>,
    pub sql_text: Option<String>,
    pub violation_detail: Option<String>,
    pub status: Option<String>,
    pub alert_count: Option<i32>,
    pub last_alert_time: Option<DateTime<Utc>>,
    pub cpu_time: Option<f64>,
    pub mem_usage: Option<i64>,
    pub exec_time: Option<f64>,
    pub scan_rows: Option<i64>,
    pub scan_bytes: Option<i64>,
    pub connection_id: Option<String>,
    pub fe_ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub remark: Option<String>,
    pub repair_person: Option<String>,
    pub ivr_msg_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<String>,
    pub cluster: Option<String>,
    pub user: Option<String>,
    pub department: Option<String>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub query_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertHistoryRemarkRequest {
    pub remark: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertHistoryRepairPersonRequest {
    pub repair_person: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertHistoryResponse {
    pub items: Vec<AlertHistory>,
    pub total: i64,
}

// Struct for show proc result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QueryProcInfo {
    pub query_id: String,
    pub connection_id: String,
    pub user: String,
    pub db: String,
    pub scan_bytes: i64,
    pub scan_rows: i64,
    pub memory_usage: i64,
    pub cpu_time: f64, // seconds
    pub exec_time: f64, // seconds
    pub sql_text: String,
    pub start_time: String, // inferred or from metadata if available? Show proc usually doesn't give start time directly in clean ISO format, might need parsing Duration
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRequest {
    pub bot_id: String,
    pub message: String,
    pub mentions: Option<Vec<String>>,
}
