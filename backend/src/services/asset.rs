use crate::models::asset::{
    ResourceAsset, ResourceAssetImport, ResourceFilterOptions, ResourceQuery,
};
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{error, info};

pub struct AssetService {
    pool: MySqlPool,
}

impl AssetService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn get_filter_options(&self) -> Result<ResourceFilterOptions> {
        let project_names = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT project_name FROM basic_information.resource WHERE project_name IS NOT NULL AND project_name != ''"
        )
        .fetch_all(&self.pool)
        .await?;

        // 1. Get manual services
        let raw_manual_services = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT manual_service FROM basic_information.resource WHERE manual_service IS NOT NULL AND manual_service != ''"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut service_set = std::collections::HashSet::new();
        for s in raw_manual_services {
            for sub in s.split(',') {
                let trimmed = sub.trim();
                if !trimmed.is_empty() {
                    service_set.insert(trimmed.to_string());
                }
            }
        }

        let mut status_set = std::collections::HashSet::new();

        // 2. Get auto services from JSON field
        let raw_auto_services: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT auto_services FROM basic_information.resource WHERE auto_services IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;

        for json_val in raw_auto_services {
            if let Some(arr) = json_val.as_array() {
                for item in arr {
                    if let Some(name) = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        service_set.insert(name.to_string());
                    }
                    if let Some(state) = item
                        .get("state")
                        .and_then(|n| n.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        status_set.insert(state.to_string());
                    }
                }
            }
        }

        let mut service_types: Vec<String> = service_set.into_iter().collect();
        service_types.sort();

        let mut service_statuses: Vec<String> = status_set.into_iter().collect();
        service_statuses.sort();

        let countries = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT country FROM basic_information.resource WHERE country IS NOT NULL AND country != ''"
        )
        .fetch_all(&self.pool)
        .await?;

        let regions = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT region FROM basic_information.resource WHERE region IS NOT NULL AND region != ''"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(ResourceFilterOptions {
            project_names,
            service_types,
            service_statuses,
            countries,
            regions,
        })
    }

    pub async fn list_resources(
        &self,
        query_params: ResourceQuery,
    ) -> Result<(Vec<ResourceAsset>, i64)> {
        let page = query_params.page.unwrap_or(1);
        let page_size = query_params.page_size.unwrap_or(20);
        let offset = (page - 1) * page_size;

        let mut conditions = Vec::new();
        let mut args = Vec::new();

        if let Some(q) = query_params.query.as_deref().filter(|s| !s.is_empty()) {
            conditions.push(
                "(instance_id LIKE ? OR instance_name LIKE ? OR public_ip LIKE ? OR private_ip LIKE ?)",
            );
            let search_param = format!("%{}%", q);
            for _ in 0..4 {
                args.push(search_param.clone());
            }
        }

        if let Some(it) = query_params
            .instance_type
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            conditions.push("instance_type = ?");
            args.push(it.to_string());
        }

        if let Some(pn) = query_params
            .project_name
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            conditions.push("project_name = ?");
            args.push(pn.to_string());
        }

        let manual_service = query_params
            .manual_service
            .as_deref()
            .filter(|s| !s.is_empty());
        let service_status = query_params
            .service_status
            .as_deref()
            .filter(|s| !s.is_empty());

        if let (Some(st), Some(ss)) = (manual_service, service_status) {
            conditions.push("JSON_CONTAINS(auto_services, JSON_OBJECT('name', ?, 'state', ?))");
            args.push(st.to_string());
            args.push(ss.to_string());
        } else {
            if let Some(st) = manual_service {
                conditions.push(
                    "(manual_service LIKE ? OR JSON_CONTAINS(auto_services, JSON_OBJECT('name', ?)))",
                );
                args.push(format!("%{}%", st));
                args.push(st.to_string());
            }

            if let Some(ss) = service_status {
                conditions.push("JSON_CONTAINS(auto_services, JSON_OBJECT('state', ?))");
                args.push(ss.to_string());
            }
        }

        if let Some(c) = query_params.country.as_deref().filter(|s| !s.is_empty()) {
            conditions.push("country = ?");
            args.push(c.to_string());
        }

        if let Some(s) = query_params.status.as_deref().filter(|s| !s.is_empty()) {
            conditions.push("status = ?");
            args.push(s.to_string());
        }

        if let Some(r) = query_params.region.as_deref().filter(|s| !s.is_empty()) {
            conditions.push("region = ?");
            args.push(r.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_query =
            format!("SELECT COUNT(*) FROM basic_information.resource {}", where_clause);
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
        for arg in &args {
            count_q = count_q.bind(arg);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        let select_query = format!(
            "SELECT * FROM basic_information.resource {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut select_q = sqlx::query_as::<_, ResourceAsset>(&select_query);
        for arg in &args {
            select_q = select_q.bind(arg);
        }
        select_q = select_q.bind(page_size).bind(offset);

        let list = select_q.fetch_all(&self.pool).await?;

        Ok((list, total))
    }

    pub async fn import_resources(&self, items: Vec<ResourceAssetImport>) -> Result<usize> {
        let mut count = 0;
        for item in items {
            let query = r#"
                INSERT INTO basic_information.resource (
                    instance_type, instance_id, instance_name, project_name, project_ownership,
                    manual_service, country, region, public_ip, private_ip, network_identifier,
                    cpu, memory, storage, `release`, remark, status
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    instance_name = VALUES(instance_name),
                    project_name = VALUES(project_name),
                    project_ownership = VALUES(project_ownership),
                    manual_service = VALUES(manual_service),
                    country = VALUES(country),
                    region = VALUES(region),
                    public_ip = VALUES(public_ip),
                    network_identifier = VALUES(network_identifier),
                    cpu = VALUES(cpu),
                    memory = VALUES(memory),
                    storage = VALUES(storage),
                    `release` = VALUES(`release`),
                    remark = VALUES(remark),
                    status = VALUES(status),
                    updated_at = NOW()
            "#;

            sqlx::query(query)
                .bind(&item.instance_type)
                .bind(&item.instance_id)
                .bind(&item.instance_name)
                .bind(&item.project_name)
                .bind(&item.project_ownership)
                .bind(&item.manual_service)
                .bind(&item.country)
                .bind(&item.region)
                .bind(&item.public_ip)
                .bind(&item.private_ip)
                .bind(&item.network_identifier)
                .bind(&item.cpu)
                .bind(&item.memory)
                .bind(&item.storage)
                .bind(&item.release)
                .bind(&item.remark)
                .bind(item.status.as_deref().unwrap_or("online"))
                .execute(&self.pool)
                .await
                .context("Failed to insert resource")?;

            count += 1;
        }

        Ok(count)
    }

    pub async fn update_resource(&self, item: ResourceAssetImport) -> Result<()> {
        let query = r#"
            UPDATE basic_information.resource
            SET instance_type = ?,
                instance_id = ?,
                instance_name = ?,
                project_name = ?,
                project_ownership = ?,
                manual_service = ?,
                country = ?,
                region = ?,
                public_ip = ?,
                private_ip = ?,
                network_identifier = ?,
                cpu = ?,
                memory = ?,
                storage = ?,
                `release` = ?,
                remark = ?,
                status = ?,
                updated_at = NOW()
            WHERE private_ip = ?
        "#;

        sqlx::query(query)
            .bind(&item.instance_type)
            .bind(&item.instance_id)
            .bind(&item.instance_name)
            .bind(&item.project_name)
            .bind(&item.project_ownership)
            .bind(&item.manual_service)
            .bind(&item.country)
            .bind(&item.region)
            .bind(&item.public_ip)
            .bind(&item.private_ip)
            .bind(&item.network_identifier)
            .bind(&item.cpu)
            .bind(&item.memory)
            .bind(&item.storage)
            .bind(&item.release)
            .bind(&item.remark)
            .bind(item.status.as_deref().unwrap_or("online"))
            .bind(&item.private_ip)
            .execute(&self.pool)
            .await
            .context("Failed to update resource")?;

        Ok(())
    }

    pub async fn delete_resources(&self, private_ips: Vec<String>) -> Result<()> {
        if private_ips.is_empty() {
            return Ok(());
        }

        let placeholders: Vec<String> = private_ips.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "DELETE FROM basic_information.resource WHERE private_ip IN ({})",
            placeholders.join(",")
        );

        let mut q = sqlx::query(&query);
        for ip in private_ips {
            q = q.bind(ip);
        }

        q.execute(&self.pool)
            .await
            .context("Failed to delete resources")?;

        Ok(())
    }

    pub async fn start_sync_loop(self: Arc<Self>) {
        info!("Starting Asset External Sync Loop (5m interval)");
        let mut interval = time::interval(Duration::from_secs(300));

        loop {
            interval.tick().await;
            if let Err(e) = self.sync_assets_from_webhook().await {
                error!("Error in asset sync loop: {}", e);
            }
        }
    }

    async fn sync_assets_from_webhook(&self) -> Result<()> {
        // 1. Get webhook URL from config
        let webhook_url: Option<String> = sqlx::query_scalar(
            "SELECT config_value FROM system_config WHERE config_key = 'asset_sync_webhook_url'",
        )
        .fetch_optional(&self.pool)
        .await?;

        let url = match webhook_url {
            Some(u) if !u.is_empty() => u,
            _ => return Ok(()),
        };

        // 2. Fetch external data
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let res = client
            .post(&url)
            .json(&json!({"type": "资产"}))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Webhook returned status {}", res.status()));
        }

        let external_data: Vec<serde_json::Value> = res.json().await?;

        // 3. Mark all existing resources as offline first
        sqlx::query("UPDATE basic_information.resource SET status = 'offline'")
            .execute(&self.pool)
            .await?;

        // 4. Process and Update assets
        for item in external_data {
            let metric = match item["metric"].as_object() {
                Some(m) => m,
                None => continue,
            };

            let ident = metric.get("ident").and_then(|v| v.as_str()).unwrap_or("");
            let private_ip = metric.get("host_ip").and_then(|v| v.as_str()).unwrap_or("");
            if private_ip.is_empty() {
                continue;
            }

            let country = metric.get("cn").and_then(|v| v.as_str());
            let cpu = metric.get("cpu").and_then(|v| v.as_str());
            let memory = metric.get("mem").and_then(|v| v.as_str());
            let storage = metric.get("total_disk").and_then(|v| v.as_str());
            let release = metric.get("os_info").and_then(|v| v.as_str());
            let service_str = metric.get("service").and_then(|v| v.as_str()).unwrap_or("");

            // Parse service string: name.service=state,name.service=state
            let mut auto_services = Vec::new();
            for s in service_str.split(',') {
                let s = s.trim();
                if s.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = s.split('=').collect();
                if parts.len() == 2 {
                    let name = parts[0].replace(".service", "");
                    let state = parts[1];
                    auto_services.push(json!({
                        "name": name,
                        "state": state
                    }));
                }
            }
            let auto_services_json = serde_json::to_value(auto_services)?;

            // Update database - status = 'online'
            let query = r#"
                INSERT INTO basic_information.resource (
                    instance_type, instance_name, private_ip, country, cpu, memory, storage, `release`, auto_services, status
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    instance_type = VALUES(instance_type),
                    instance_name = VALUES(instance_name),
                    country = VALUES(country),
                    cpu = VALUES(cpu),
                    memory = VALUES(memory),
                    storage = VALUES(storage),
                    `release` = VALUES(`release`),
                    auto_services = VALUES(auto_services),
                    status = 'online',
                    updated_at = NOW()
            "#;

            sqlx::query(query)
                .bind("服务器")
                .bind(ident)
                .bind(private_ip)
                .bind(country)
                .bind(cpu)
                .bind(memory)
                .bind(storage)
                .bind(release)
                .bind(auto_services_json)
                .bind("online")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn apply_resources(
        &self,
        req: crate::models::asset::ResourceApplyRequest,
    ) -> Result<crate::models::asset::ResourceApplyResponse> {
        use crate::models::asset::ResourceApplyResponse;
        use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue};

        let mut success_count = 0;
        let mut failed_ips = Vec::new();
        let mut not_found_ips = Vec::new();

        let base_url = "https://example.com";
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(COOKIE, HeaderValue::from_str(&req.cookie)?);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // 1. Get personal info
        let info_url = format!("{}/a/common/query/info", base_url);
        let info_payload = serde_json::json!({
            "type": "personal_information",
            "data": {}
        });

        let info_res = client
            .post(&info_url)
            .headers(headers.clone())
            .json(&info_payload)
            .send()
            .await?;

        if !info_res.status().is_success() {
            return Ok(ResourceApplyResponse {
                total_count: req.ip_list.len(),
                success_count: 0,
                failed_ips: req.ip_list.clone(),
                not_found_ips: Vec::new(),
                error_msg: Some(format!(
                    "获取个人信息异常: {} {}",
                    info_res.status(),
                    info_res.text().await.unwrap_or_default()
                )),
            });
        }

        let info_data: serde_json::Value = info_res.json().await?;
        if !info_data["success"].as_bool().unwrap_or(false) {
            return Ok(ResourceApplyResponse {
                total_count: req.ip_list.len(),
                success_count: 0,
                failed_ips: req.ip_list.clone(),
                not_found_ips: Vec::new(),
                error_msg: Some(format!(
                    "获取个人信息失败: {}",
                    info_data["msg"].as_str().unwrap_or("未知错误")
                )),
            });
        }

        let u_data = &info_data["data"];
        let user_info = serde_json::json!({
            "userId": u_data["id"],
            "userName": u_data["name"],
            "userEmail": u_data["email"],
            "user_email": u_data["email"],
            "officeName": u_data["office"]["name"],
            "companyName": u_data["company"]["name"],
            "remarks": req.remarks
        });

        // 2. Query instances and projects
        let mut apply_items = Vec::new();

        for ip in &req.ip_list {
            // Get instance
            let inst_payload = serde_json::json!({
                "type": "instance_list_by_type_and_ip",
                "data": { "instanceType": "vm", "ip": ip }
            });

            let inst_res = client
                .post(&info_url)
                .headers(headers.clone())
                .json(&inst_payload)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            if !inst_res["success"].as_bool().unwrap_or(false)
                || inst_res["data"].as_array().is_none_or(|a| a.is_empty())
            {
                not_found_ips.push(ip.clone());
                continue;
            }

            let instance = &inst_res["data"][0];
            let instance_id = instance["value"].as_str().unwrap_or_default().to_string();
            let instance_name = instance["label"].as_str().unwrap_or_default().to_string();

            // Get project
            let proj_payload = serde_json::json!({
                "type": "project_info",
                "data": { "instanceId": instance_id }
            });

            let proj_res = client
                .post(&info_url)
                .headers(headers.clone())
                .json(&proj_payload)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            if !proj_res["success"].as_bool().unwrap_or(false)
                || proj_res["data"].as_array().is_none_or(|a| a.is_empty())
            {
                not_found_ips.push(ip.clone());
                continue;
            }

            let project = &proj_res["data"][0];
            apply_items.push(serde_json::json!({
                "ip": ip,
                "instance_id": instance_id,
                "instanceName": instance_name,
                "project": project["value"],
                "projectName": project["label"]
            }));
        }

        // 3. Batch apply
        if apply_items.is_empty() {
            return Ok(ResourceApplyResponse {
                total_count: req.ip_list.len(),
                success_count: 0,
                failed_ips: Vec::new(),
                not_found_ips,
                error_msg: Some("未找到任何有效实例".to_string()),
            });
        }

        let submit_url = format!("{}/a/common/submit/info", base_url);
        for chunk in apply_items.chunks(10) {
            let mut request_list = Vec::new();
            for (i, item) in chunk.iter().enumerate() {
                request_list.push(serde_json::json!({
                    "type": "jump_authorized_assets_permissions",
                    "typeName": "JumpServer机器授权",
                    "data": {
                        "userId": user_info["userId"],
                        "userName": user_info["userName"],
                        "userEmail": user_info["userEmail"],
                        "projectName": item["projectName"],
                        "instanceName": item["instanceName"],
                        "user_email": user_info["user_email"],
                        "companyName": user_info["companyName"],
                        "officeName": user_info["officeName"],
                        "instance_id": item["instance_id"],
                        "project": item["project"],
                        "system_user": "root",
                        "systemUser": "root",
                        "expire_date": 1095,
                        "expireDate": "7天",
                        "remarks": user_info["remarks"],
                        "subButton": null,
                        "batchApplyId": i + 1
                    },
                    "isExpanded": false,
                    "msg": ""
                }));
            }

            let batch_payload = serde_json::json!({
                "type": "batch_apply",
                "data": { "list": request_list }
            });

            let batch_res = client
                .post(&submit_url)
                .headers(headers.clone())
                .json(&batch_payload)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            if batch_res["success"].as_bool().unwrap_or(false) {
                success_count += chunk.len();
            } else {
                for item in chunk {
                    failed_ips.push(item["ip"].as_str().unwrap_or_default().to_string());
                }
            }

            // Optional delay if needed, but for server side it might be better to just return or use background task.
            // However, the user script had a 30s delay between batches.
            // For now, let's keep it simple.
        }

        Ok(ResourceApplyResponse {
            total_count: req.ip_list.len(),
            success_count,
            failed_ips,
            not_found_ips,
            error_msg: None,
        })
    }

    pub async fn service_operation(
        &self,
        req: crate::models::asset::ResourceServiceOpRequest,
    ) -> Result<String> {
        let webhook_url: Option<String> = sqlx::query_scalar(
            "SELECT config_value FROM system_config WHERE config_key = 'asset_sync_webhook_url'",
        )
        .fetch_optional(&self.pool)
        .await?;

        let url = match webhook_url {
            Some(u) if !u.is_empty() => u,
            _ => return Err(anyhow::anyhow!("Webhook URL not configured")),
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let payload = json!({
            "type": req.op_type,
            "service": req.service,
            "ip": req.ip
        });

        let res = client.post(&url).json(&payload).send().await?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Webhook returned status {}", res.status()));
        }

        let text = res.text().await?;

        // Try to parse as JSON to extract stdout/msg in a friendly way
        if let Ok(json_val) = serde_json::from_str::<Value>(&text) {
            let mut combined_output = String::new();
            let ansible_re = Regex::new(r"(?m)^.* \| .* \| rc=\d+ >>\n?").unwrap();

            let mut process_item = |item: &Value| {
                let content = item
                    .get("stdout")
                    .and_then(|v| v.as_str())
                    .or_else(|| item.get("msg").and_then(|v| v.as_str()));

                if let Some(c) = content {
                    let cleaned = ansible_re.replace_all(c, "");
                    if !combined_output.is_empty() {
                        combined_output.push_str("\n---\n");
                    }
                    combined_output.push_str(&cleaned);
                }
            };

            if let Some(arr) = json_val.as_array() {
                for item in arr {
                    process_item(item);
                }
            } else if json_val.is_object() {
                process_item(&json_val);
            }

            if !combined_output.is_empty() {
                return Ok(combined_output);
            }
        }

        Ok(text)
    }
}
