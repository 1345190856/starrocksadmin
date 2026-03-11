use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct DutyPersonnel {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub org_lvl1: Option<String>,
    #[serde(default)]
    pub org_lvl2: Option<String>,
    pub email: String,
    pub phone: String,
    pub duty_platform: Option<String>,
    pub responsible_domain: Option<String>,
    pub countries: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateDutyPersonnelRequest {
    pub name: String,
    pub org_lvl1: Option<String>,
    pub org_lvl2: Option<String>,
    pub email: String,
    pub phone: String,
    pub duty_platform: Option<String>,
    pub responsible_domain: Option<String>,
    pub countries: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateDutyPersonnelRequest {
    pub name: Option<String>,
    pub org_lvl1: Option<String>,
    pub org_lvl2: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub duty_platform: Option<String>,
    pub responsible_domain: Option<String>,
    pub countries: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct DutySchedule {
    pub id: i32,
    pub duty_date: NaiveDate,
    pub country: String,
    pub duty_platform: Option<String>,
    pub shift_type: String,
    pub personnel_id: i32,
    // Joined fields from personnel
    #[sqlx(default)]
    pub personnel_name: Option<String>,
    #[sqlx(default)]
    pub personnel_email: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DutyScheduleItem {
    pub duty_date: NaiveDate,
    pub country: String,
    pub shift_type: String, // 09:00-21:00, 21:00-09:00, All Day
    pub personnel_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchAssignDutyRequest {
    pub schedules: Vec<DutyScheduleItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
pub struct DutyScheduleQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub country: Option<String>,
    pub duty_platform: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct DutyRotation {
    pub id: i32,
    pub name: String,
    pub personnel_ids: String, // JSON array string
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub period_days: Option<i32>,
    pub country: Option<String>,
    pub bot_ids: Option<String>,
    pub auto_notify: Option<bool>,
    pub notify_advance_hours: Option<i32>,
    pub last_notified_date: Option<NaiveDate>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateDutyRotationRequest {
    pub name: String,
    pub personnel_ids: Vec<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub period_days: i32,
    pub country: String,
    pub bot_ids: Option<String>,
    pub auto_notify: Option<bool>,
    pub notify_advance_hours: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateRotationConfigRequest {
    pub name: String,
    pub bot_ids: Option<String>,
    pub auto_notify: bool,
    pub notify_advance_hours: i32,
}
