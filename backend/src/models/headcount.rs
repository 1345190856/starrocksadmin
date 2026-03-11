use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct Employee {
    pub id: i32, // Manual ID from OA
    #[serde(rename = "userId")]
    #[sqlx(rename = "userId")]
    pub user_id: String,
    pub user: Option<String>,
    pub name: String,
    pub city: Option<String>,
    pub company: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "employeeNumber")]
    #[sqlx(rename = "employeeNumber")]
    pub employee_number: Option<String>,
    pub phone: Option<String>,
    #[serde(rename = "orgFullPath")]
    #[sqlx(rename = "orgFullPath")]
    pub org_full_path: Option<String>,
    pub position: Option<String>,
    #[serde(rename = "join_at")]
    pub join_at: Option<String>,
    #[serde(rename = "leaveAt")]
    #[sqlx(rename = "leaveAt")]
    pub leave_at: Option<String>,
    #[serde(rename = "labor_type_txt")]
    pub labor_type_txt: Option<String>,
    #[serde(rename = "status_txt")]
    pub status_txt: Option<String>,
    #[serde(rename = "orgName_1")]
    #[sqlx(rename = "orgName_1")]
    pub org_name_1: Option<String>,
    #[serde(rename = "orgName_2")]
    #[sqlx(rename = "orgName_2")]
    pub org_name_2: Option<String>,
    #[serde(rename = "orgName")]
    #[sqlx(rename = "orgName")]
    pub org_name: Option<String>,
    #[serde(rename = "position_level_mame")]
    pub position_level_mame: Option<String>,
    #[serde(rename = "leaderEmployeeNumber")]
    #[sqlx(rename = "leaderEmployeeNumber")]
    pub leader_employee_number: Option<String>,
    #[serde(rename = "leaderId")]
    #[sqlx(rename = "leaderId")]
    pub leader_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OATokenResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct OAEmployeeWrapper {
    pub code: i32,
    #[serde(alias = "message")]
    pub msg: Option<String>,
    pub data: Vec<Employee>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct EmployeeQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub query: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmployeeListResponse {
    pub list: Vec<Employee>,
    pub total: i64,
}
