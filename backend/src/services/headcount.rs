use crate::models::headcount::{Employee, OAEmployeeWrapper, OATokenResponse};
use anyhow::{Context, Result};
use reqwest::Client;
use sqlx::MySqlPool;
use tracing::{error, info};

pub struct HeadcountService {
    pool: MySqlPool,
    client: Client,
}

impl HeadcountService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool, client: Client::new() }
    }

    pub async fn fetch_token(&self) -> Result<String> {
        let url = "https://example.com/oauth/token";
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", "32"),
            ("client_secret", "oYcO11WxhJfHImkxuugztivyPxY8oX7fpPdMRqEG"),
            ("scope", "organization"),
        ];

        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .context("Failed to send token request")?
            .json::<OATokenResponse>()
            .await
            .context("Failed to parse token response")?;

        Ok(resp.access_token)
    }

    pub async fn fetch_employees_from_oa(&self, token: &str) -> Result<Vec<Employee>> {
        let url = "http://example.com/openapi/employee/getEmployeeListWithBasicInfo";

        let resp = self
            .client
            .get(url)
            .query(&[("user_number", ""), ("scope", "organization")])
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to fetch employee list")?;

        let text = resp.text().await.context("Failed to get response text")?;

        // Debug log (optional, remove in production if too verbose, but useful now)
        // debug!("OA Response: {}", text);

        let wrapper: OAEmployeeWrapper = match serde_json::from_str(&text) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to parse OA response. Error: {}. Response: {}", e, text);
                return Err(anyhow::anyhow!("Failed to parse OA response: {}", e));
            },
        };

        if wrapper.code != 0 {
            return Err(anyhow::anyhow!(
                "OA API Error: {}",
                wrapper.msg.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(wrapper.data)
    }

    pub async fn sync_employees(&self) -> Result<usize> {
        let token = self.fetch_token().await?;
        let employees = self.fetch_employees_from_oa(&token).await?;
        let count = employees.len();

        info!("Synced {} employees from OA", count);

        for emp in employees {
            let query = r#"
                INSERT INTO basic_information.employee (
                    id, userId, user, name, city, company, email, employeeNumber, phone, 
                    orgFullPath, position, join_at, leaveAt, labor_type_txt, status_txt,
                    orgName_1, orgName_2, orgName, position_level_mame, leaderEmployeeNumber, leaderId
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    userId = VALUES(userId),
                    user = VALUES(user),
                    name = VALUES(name),
                    city = VALUES(city),
                    company = VALUES(company),
                    email = VALUES(email),
                    employeeNumber = VALUES(employeeNumber),
                    phone = VALUES(phone),
                    orgFullPath = VALUES(orgFullPath),
                    position = VALUES(position),
                    join_at = VALUES(join_at),
                    leaveAt = VALUES(leaveAt),
                    labor_type_txt = VALUES(labor_type_txt),
                    status_txt = VALUES(status_txt),
                    orgName_1 = VALUES(orgName_1),
                    orgName_2 = VALUES(orgName_2),
                    orgName = VALUES(orgName),
                    position_level_mame = VALUES(position_level_mame),
                    leaderEmployeeNumber = VALUES(leaderEmployeeNumber),
                    leaderId = VALUES(leaderId),
                    updated_at = NOW()
            "#;

            sqlx::query(query)
                .bind(emp.id)
                .bind(&emp.user_id)
                .bind(format!("u_{}", emp.user_id))
                .bind(&emp.name)
                .bind(&emp.city)
                .bind(&emp.company)
                .bind(&emp.email)
                .bind(emp.employee_number.as_deref().unwrap_or(""))
                .bind(&emp.phone)
                .bind(&emp.org_full_path)
                .bind(emp.position.as_deref().unwrap_or(""))
                .bind(&emp.join_at)
                .bind(&emp.leave_at)
                .bind(&emp.labor_type_txt)
                .bind(&emp.status_txt)
                .bind(&emp.org_name_1)
                .bind(&emp.org_name_2)
                .bind(&emp.org_name)
                .bind(&emp.position_level_mame)
                .bind(&emp.leader_employee_number)
                .bind(emp.leader_id)
                .execute(&self.pool)
                .await
                .context(format!("Failed to upsert employee {}", emp.id))?;
        }

        Ok(count)
    }

    pub async fn list_employees(
        &self,
        page: i64,
        page_size: i64,
        query: Option<String>,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<(Vec<Employee>, i64)> {
        let offset = (page - 1) * page_size;
        let search = query.unwrap_or_default();
        let search_param = format!("%{}%", search);

        let count_query = r#"
            SELECT COUNT(*) FROM basic_information.employee 
            WHERE 
                userId LIKE ? OR name LIKE ? OR email LIKE ? OR employeeNumber LIKE ? OR phone LIKE ?
        "#;

        let total: i64 = sqlx::query_scalar(count_query)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .fetch_one(&self.pool)
            .await?;

        // Handle sorting
        let allowed_sort_fields = ["id", "join_at", "leaveAt", "status_txt", "position_level_mame"];
        let sort_column = if let Some(field) = sort_by {
            if allowed_sort_fields.contains(&field.as_str()) { field } else { "id".to_string() }
        } else {
            "id".to_string()
        };

        let direction = match sort_order.as_deref() {
            Some("desc") => "DESC",
            _ => "ASC",
        };

        let select_query = format!(
            r#"
            SELECT * FROM basic_information.employee 
            WHERE 
                userId LIKE ? OR name LIKE ? OR email LIKE ? OR employeeNumber LIKE ? OR phone LIKE ?
            ORDER BY {} {}
            LIMIT ? OFFSET ?
            "#,
            sort_column, direction
        );

        let employees = sqlx::query_as::<_, Employee>(&select_query)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .bind(&search_param)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((employees, total))
    }
}
