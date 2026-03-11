use crate::models::alert::{
    AlertHistoryResponse, AlertRule, CreateAlertRuleRequest, HistoryQuery, NotificationRequest,
    UpdateAlertRuleRequest,
};
use crate::{AppState, utils::ApiResult};
use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;

#[derive(serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct TrendQuery {
    pub days: Option<u32>,
}

// Rules
#[utoipa::path(
    get,
    path = "/api/alert/rules",
    tag = "Alerts",
    responses(
        (status = 200, description = "List alert rules", body = Vec<AlertRule>)
    )
)]
pub async fn list_rules(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<AlertRule>>> {
    let rules = state.alert_service.list_rules().await?;
    Ok(Json(rules))
}

#[utoipa::path(
    post,
    path = "/api/alert/rules",
    tag = "Alerts",
    request_body = CreateAlertRuleRequest,
    responses(
        (status = 200, description = "Create alert rule", body = AlertRule)
    )
)]
pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAlertRuleRequest>,
) -> ApiResult<Json<AlertRule>> {
    let rule = state.alert_service.create_rule(req).await?;
    Ok(Json(rule))
}

#[utoipa::path(
    put,
    path = "/api/alert/rules/{id}",
    tag = "Alerts",
    request_body = UpdateAlertRuleRequest,
    responses(
        (status = 200, description = "Update alert rule", body = AlertRule)
    )
)]
pub async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateAlertRuleRequest>,
) -> ApiResult<Json<AlertRule>> {
    let rule = state.alert_service.update_rule(id, req).await?;
    Ok(Json(rule))
}

#[utoipa::path(
    delete,
    path = "/api/alert/rules/{id}",
    tag = "Alerts",
    responses(
        (status = 200, description = "Delete alert rule")
    )
)]
pub async fn delete_rule(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> ApiResult<()> {
    state.alert_service.delete_rule(id).await?;
    Ok(())
}

// History
#[utoipa::path(
    get,
    path = "/api/alert/history",
    tag = "Alerts",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("pageSize" = Option<u32>, Query, description = "Items per page"),
        ("status" = Option<String>, Query, description = "Filter by status")
    ),
    responses(
        (status = 200, description = "List alert history", body = AlertHistoryResponse)
    )
)]
pub async fn list_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> ApiResult<Json<AlertHistoryResponse>> {
    let history = state.alert_service.list_history(query).await?;
    Ok(Json(history))
}

#[utoipa::path(
    get,
    path = "/api/alert/summary/sql",
    tag = "Alerts",
    responses(
        (status = 200, description = "Get SQL alert summary")
    )
)]
pub async fn get_sql_summary(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<serde_json::Value>> {
    let summary = state.alert_service.get_sql_alert_summary().await?;
    Ok(Json(summary))
}

#[utoipa::path(
    get,
    path = "/api/alert/summary/sql/trend",
    tag = "Alerts",
    responses(
        (status = 200, description = "Get SQL alert history trend")
    )
)]
pub async fn get_sql_trend(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TrendQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let trend = state
        .alert_service
        .get_sql_alert_trend(query.days.unwrap_or(30))
        .await?;
    Ok(Json(trend))
}

#[utoipa::path(
    get,
    path = "/api/alert/summary/external",
    tag = "Alerts",
    responses(
        (status = 200, description = "Get synced external alert statistics")
    )
)]
pub async fn get_external_summary(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<serde_json::Value>> {
    use crate::utils::ApiError;
    use sqlx::Row;
    let rows = sqlx::query("SELECT group_name, active_alert, today_alert, trend_data, change_rate FROM alert_external_statistics")
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::database_error)?;

    tracing::info!("get_external_summary: found {} rows", rows.len());

    let mut result = serde_json::Map::new();
    for row in rows {
        let group_name: String = row.get("group_name");

        // Use try_get to avoid panics and get better error messages
        let active_alert: i32 = row.try_get("active_alert").unwrap_or(0);
        let today_alert: i32 = row.try_get("today_alert").unwrap_or(0);
        let trend_data: serde_json::Value = row.try_get("trend_data").unwrap_or(json!([]));

        // Read change_rate as f64 (Double)
        let change_rate: f64 = row.try_get("change_rate").unwrap_or(0.0);

        tracing::info!("Group: {}, today: {}, change: {}", group_name, today_alert, change_rate);

        result.insert(
            group_name,
            json!({
                "active_alert": active_alert,
                "today_alert": today_alert,
                "trend_data": trend_data,
                "change_rate": change_rate
            }),
        );
    }

    Ok(Json(serde_json::Value::Object(result)))
}

#[utoipa::path(
    post,
    path = "/api/alert/summary/external",
    tag = "Alerts",
    responses(
        (status = 200, description = "Proxy external alert data")
    )
)]
pub async fn proxy_webhook(
    State(state): State<Arc<AppState>>,
    body: String,
) -> ApiResult<Json<serde_json::Value>> {
    let config: Option<crate::models::system::SystemConfig> =
        state.system_service.get_config("alert_webhook_url").await?;
    let url = match config {
        Some(c) if !c.config_value.is_empty() => c.config_value,
        _ => return Err(crate::utils::ApiError::invalid_data("Alert webhook URL not configured")),
    };

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| crate::utils::ApiError::internal_error(format!("Webhook error: {}", e)))?;

    let text = res.text().await.map_err(|e| {
        crate::utils::ApiError::internal_error(format!("Failed to read webhook response: {}", e))
    })?;

    if text.trim().is_empty() {
        return Ok(Json(json!({})));
    }

    let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        crate::utils::ApiError::internal_error(format!("Webhook JSON error (Raw: {}): {}", text, e))
    })?;

    Ok(Json(data))
}

#[utoipa::path(
    get,
    path = "/api/alert/history/clusters",
    tag = "Alerts",
    responses(
        (status = 200, description = "List unique clusters in alert history", body = Vec<String>)
    )
)]
pub async fn list_history_clusters(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<String>>> {
    let clusters = state.alert_service.get_history_clusters().await?;
    Ok(Json(clusters))
}

#[utoipa::path(
    get,
    path = "/api/alert/history/departments",
    tag = "Alerts",
    responses(
        (status = 200, description = "List unique departments in alert history", body = Vec<String>)
    )
)]
pub async fn list_history_departments(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<String>>> {
    let departments = state.alert_service.get_history_departments().await?;
    Ok(Json(departments))
}

#[utoipa::path(
    post,
    path = "/api/alert/rules/{id}/test",
    params(("id" = i32, Path, description = "Rule ID")),
    responses((status = 200, description = "Test alert sent"))
)]
pub async fn test_alert(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<impl IntoResponse> {
    state.alert_service.test_alert(id).await?;
    Ok(Json(json!({ "message": "Test alert sent" })))
}

#[utoipa::path(
    post,
    path = "/api/alert/notify",
    request_body = NotificationRequest,
    responses((status = 200, description = "Notification sent"))
)]
pub async fn notify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NotificationRequest>,
) -> ApiResult<impl IntoResponse> {
    let mentions = req.mentions.unwrap_or_default();
    state
        .alert_service
        .send_notification(&req.bot_id, &req.message, mentions)
        .await?;
    Ok(Json(json!({ "message": "Notification sent" })))
}

#[utoipa::path(
    get,
    path = "/api/alert/history/{id}",
    tag = "Alerts",
    params(
        ("id" = i32, Path, description = "History ID")
    ),
    responses(
        (status = 200, description = "Get alert history detail", body = AlertHistory)
    )
)]
pub async fn get_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<crate::models::alert::AlertHistory>> {
    let mut history = state.alert_service.get_history_by_id(id).await?;
    state.alert_service.ensure_sql_text(&mut history).await?;
    Ok(Json(history))
}

#[utoipa::path(
    post,
    path = "/api/alert/history/{id}/kill",
    params(("id" = i32, Path, description = "History ID")),
    responses((status = 200, description = "Query killed"))
)]
pub async fn kill_query(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<impl IntoResponse> {
    state.alert_service.kill_query(id).await?;
    Ok(Json(json!({ "message": "Query killed" })))
}

#[utoipa::path(
    post,
    path = "/api/alert/history/{id}/whitelist",
    params(("id" = i32, Path, description = "History ID")),
    responses((status = 200, description = "Query whitelisted"))
)]
pub async fn whitelist_query(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<impl IntoResponse> {
    state.alert_service.whitelist_query(id).await?;
    Ok(Json(json!({ "message": "Query whitelisted" })))
}

#[utoipa::path(
    put,
    path = "/api/alert/history/{id}/remark",
    params(("id" = i32, Path, description = "History ID")),
    request_body = UpdateAlertHistoryRemarkRequest,
    responses((status = 200, description = "Remark updated"))
)]
pub async fn update_remark(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<crate::models::alert::UpdateAlertHistoryRemarkRequest>,
) -> ApiResult<impl IntoResponse> {
    state.alert_service.update_remark(id, req.remark).await?;
    Ok(Json(json!({ "message": "Remark updated" })))
}

#[utoipa::path(
    put,
    path = "/api/alert/history/{id}/repair_person",
    params(("id" = i32, Path, description = "History ID")),
    request_body = UpdateAlertHistoryRepairPersonRequest,
    responses((status = 200, description = "Repair person updated"))
)]
pub async fn update_repair_person(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<crate::models::alert::UpdateAlertHistoryRepairPersonRequest>,
) -> ApiResult<impl IntoResponse> {
    state
        .alert_service
        .update_repair_person(id, req.repair_person)
        .await?;
    Ok(Json(json!({ "message": "Repair person updated" })))
}

pub async fn get_shared_sql(
    State(state): State<Arc<AppState>>,
    Path(query_id): Path<String>,
) -> impl IntoResponse {
    let query_id = query_id.trim();
    // Handle potential trailing characters from link parsers (e.g. ')')
    let query_id = query_id.trim_end_matches(')');

    match state.alert_service.get_history_by_query_id(query_id).await {
        Ok(mut history) => {
            let _ = state.alert_service.ensure_sql_text(&mut history).await;
            let sql = history.sql_text.as_deref().unwrap_or("No SQL content");
            let status = history.status.as_deref().unwrap_or("Unknown");

            let status_cn = match status {
                "Resolved" => "已结束",
                "Killed" => "已强杀",
                "Alerting" => "告警中",
                "Whitelisted" => "已加白",
                _ => status,
            };
            let status_html = format!(
                "<div style='color:#8f9bb3;margin-bottom:15px;font-size:14px;'>当前状态: <span style='color:#00d68f'>{}</span></div>",
                status_cn
            );

            let action_buttons = if status == "Alerting" || status == "Whitelisted" {
                let kill_btn = format!(
                    r#"<button id="killBtn" onclick="confirmKill('{}')" style='background:#ff3d71;color:white;border:none;padding:10px 20px;border-radius:4px;cursor:pointer;font-weight:bold;'>KILL QUERY</button>"#,
                    query_id
                );
                let white_btn = if status == "Alerting" {
                    format!(
                        r#"<button id="whiteBtn" onclick="whitelist('{}')" style='background:#3366ff;color:white;border:none;padding:10px 20px;border-radius:4px;cursor:pointer;font-weight:bold;'>加白 (REMAIN RUNNING)</button>"#,
                        query_id
                    )
                } else {
                    "".to_string()
                };

                format!(
                    r#"{}
                    <div style='display:flex;gap:10px;margin-bottom:15px;'>
                        {}
                        {}
                    </div>
                    <script>
                    function confirmKill(qid) {{
                        if (confirm("确定要杀掉这个查询吗？")) {{
                            const btn = document.getElementById('killBtn');
                            btn.disabled = true;
                            btn.innerText = 'Killing...';
                            fetch('/share/sql/' + qid + '/kill', {{ 
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/json' }}
                            }})
                            .then(async res => {{
                                const data = await res.json();
                                if (res.ok) {{
                                    alert(data.message || '指令已成功发送');
                                    location.reload();
                                }} else {{
                                    alert('操作失败: ' + (data.message || '未知错误'));
                                    btn.disabled = false;
                                    btn.innerText = 'KILL QUERY';
                                }}
                            }})
                            .catch(err => {{
                                alert('网络错误: ' + err);
                                btn.disabled = false;
                                btn.innerText = 'KILL QUERY';
                            }});
                        }}
                    }}
                    function whitelist(qid) {{
                        if (confirm("确定要将此查询加入白名单吗？(不再告警，但继续运行)")) {{
                            const btn = document.getElementById('whiteBtn');
                            btn.disabled = true;
                            btn.innerText = 'Processing...';
                            fetch('/share/sql/' + qid + '/whitelist', {{ 
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/json' }}
                            }})
                            .then(async res => {{
                                const data = await res.json();
                                if (res.ok) {{
                                    alert(data.message || '已成功加入白名单');
                                    location.reload();
                                }} else {{
                                    alert('操作失败: ' + (data.message || '未知错误'));
                                    btn.disabled = false;
                                    btn.innerText = '加白 (REMAIN RUNNING)';
                                }}
                            }})
                            .catch(err => {{
                                alert('网络错误: ' + err);
                                btn.disabled = false;
                                btn.innerText = '加白 (REMAIN RUNNING)';
                            }});
                        }}
                    }}
                    </script>"#,
                    status_html, kill_btn, white_btn
                )
            } else {
                status_html
            };

            axum::response::Html(format!(
                "<html><head><title>SQL Detail - {}</title></head><body style='background:#1a1a1a;color:#eee;font-family:sans-serif;padding:20px;'>
                <h3 style='color:#00d1b2;margin-top:0;'>StarRocks SQL Detail</h3>
                <div style='color:#8f9bb3;margin-bottom:10px;font-size:12px;'>Query ID: {}</div>
                {}
                <pre style='white-space:pre-wrap;word-break:break-all;border:1px solid #444;padding:15px;background:#222;font-family:monospace;line-height:1.5;border-radius:4px;'>{}</pre>
                </body></html>",
                query_id, query_id, action_buttons, sql
            )).into_response()
        },
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "SQL not found").into_response(),
    }
}

pub async fn whitelist_shared_query(
    State(state): State<Arc<AppState>>,
    Path(query_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .alert_service
        .whitelist_query_by_query_id(&query_id)
        .await?;
    Ok(Json(json!({ "message": "Query whitelisted" })))
}
pub async fn kill_shared_query(
    State(state): State<Arc<AppState>>,
    Path(query_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    state
        .alert_service
        .kill_query_by_query_id(&query_id)
        .await?;
    Ok(Json(json!({ "message": "Query killed" })))
}
