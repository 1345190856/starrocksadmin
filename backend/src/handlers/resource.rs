use std::sync::Arc;
use axum::{Json, extract::{Path, State}};
use crate::{AppState, models::resource::*, utils::{ApiError, ApiResult}};
use reqwest::Client;
use serde_json::Value;
use mysql_async::prelude::Queryable;

// --- Panels ---

#[utoipa::path(
    get,
    path = "/api/resource/panels",
    responses((status = 200, body = Vec<ResourcePanel>)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn list_panels(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<ResourcePanel>>> {
    let panels = sqlx::query_as::<_, ResourcePanel>("SELECT * FROM resource_panels ORDER BY section, display_order")
        .fetch_all(&state.db).await?;
    Ok(Json(panels))
}

#[utoipa::path(
    post,
    path = "/api/resource/panels",
    request_body = CreatePanelRequest,
    responses((status = 201, body = ResourcePanel)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn create_panel(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreatePanelRequest>,
) -> ApiResult<Json<ResourcePanel>> {
    let id = sqlx::query(
        "INSERT INTO resource_panels (section, title, chart_type, promql_query, config, display_order, data_source_id, country) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(payload.section.clone().unwrap_or_else(|| "cluster".to_string()))
    .bind(&payload.title)
    .bind(&payload.chart_type)
    .bind(&payload.promql_query)
    .bind(&payload.config)
    .bind(payload.display_order.unwrap_or(0))
    .bind(payload.data_source_id)
    .bind(payload.country)
    .execute(&state.db).await?
    .last_insert_id();

    let panel = sqlx::query_as::<_, ResourcePanel>("SELECT * FROM resource_panels WHERE id = ?")
        .bind(id).fetch_one(&state.db).await?;
    Ok(Json(panel))
}

#[utoipa::path(
    put,
    path = "/api/resource/panels/{id}",
    request_body = UpdatePanelRequest,
    responses((status = 200, body = ResourcePanel)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn update_panel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdatePanelRequest>,
) -> ApiResult<Json<ResourcePanel>> {
    let mut current = sqlx::query_as::<_, ResourcePanel>("SELECT * FROM resource_panels WHERE id = ?")
        .bind(id).fetch_one(&state.db).await
        .map_err(|_| ApiError::not_found("Panel not found"))?;

    if let Some(t) = payload.title { current.title = t; }
    if let Some(c) = payload.chart_type { current.chart_type = c; }
    if let Some(p) = payload.promql_query { current.promql_query = p; }
    if let Some(c) = payload.config { current.config = Some(c); }
    if let Some(o) = payload.display_order { current.display_order = o; }
    if let Some(d) = payload.data_source_id { current.data_source_id = Some(d); }
    if let Some(c) = payload.country { current.country = Some(c); }

    sqlx::query(
        "UPDATE resource_panels SET title=?, chart_type=?, promql_query=?, config=?, display_order=?, data_source_id=?, country=? WHERE id=?"
    )
    .bind(&current.title)
    .bind(&current.chart_type)
    .bind(&current.promql_query)
    .bind(&current.config)
    .bind(current.display_order)
    .bind(current.data_source_id)
    .bind(&current.country)
    .bind(id)
    .execute(&state.db).await?;

    Ok(Json(current))
}

#[utoipa::path(
    delete,
    path = "/api/resource/panels/{id}",
    responses((status = 200, description = "Deleted")),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn delete_panel(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> ApiResult<Json<()>> {
    sqlx::query("DELETE FROM resource_panels WHERE id = ?").bind(id).execute(&state.db).await?;
    Ok(Json(()))
}

// --- Data Sources ---

#[utoipa::path(
    get,
    path = "/api/resource/datasources",
    responses((status = 200, body = Vec<ResourceDataSource>)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn list_datasources(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<ResourceDataSource>>> {
    let ds = sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources ORDER BY id DESC")
        .fetch_all(&state.db).await?;
    Ok(Json(ds))
}

#[utoipa::path(
    post,
    path = "/api/resource/datasources/test",
    request_body = TestDataSourceRequest,
    responses((status = 200, body = Value)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
#[allow(clippy::collapsible_if)]
pub async fn test_datasource(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TestDataSourceRequest>,
) -> ApiResult<Json<Value>> {
    let mut password = payload.password.clone();
    
    if password.is_none() || password.as_ref().unwrap().is_empty() {
        if let Some(id) = payload.id {
             let ds = sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources WHERE id = ?")
                .bind(id).fetch_optional(&state.db).await?;
            if let Some(d) = ds {
                password = d.password;
            }
        }
    }

    if payload.r#type == "prometheus" {
        let client = Client::builder().timeout(std::time::Duration::from_secs(5)).build()
            .map_err(|e| ApiError::internal_error(format!("Client build failed: {}", e)))?;
        
        let mut base_url = payload.url.trim_end_matches('/').to_string();
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            base_url = format!("http://{}", base_url);
        }
        
        // Try query API for better check than /-/healthy if not available, but healthy is standard
        // Use /-/healthy first
        let url = format!("{}/-/healthy", base_url);
        
        let mut builder = client.get(&url);
        if let (Some(u), Some(p)) = (&payload.username, &password) {
            builder = builder.basic_auth(u, Some(p));
        }
        
        match builder.send().await {
            Ok(r) => {
                if r.status().is_success() {
                    Ok(Json(serde_json::json!({ "status": "success", "message": "Connection successful" })))
                } else {
                     // Try fallback to /api/v1/status/buildinfo
                     let fallback_url = format!("{}/api/v1/status/buildinfo", base_url);
                     let mut builder2 = client.get(&fallback_url);
                     if let (Some(u), Some(p)) = (&payload.username, &password) { builder2 = builder2.basic_auth(u, Some(p)); }
                     match builder2.send().await {
                        Ok(r2) => {
                             if r2.status().is_success() {
                                 Ok(Json(serde_json::json!({ "status": "success", "message": "Connection successful (via API)" })))
                             } else {
                                 Ok(Json(serde_json::json!({ "status": "error", "message": format!("HTTP Error: {}", r.status()) })))
                             }
                        },
                        Err(_) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("HTTP Error: {}", r.status()) })))
                     }
                }
            }
            // This error message format is what the user likely saw
            Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("Connection failed: {}", e) })))
        }
    } else if payload.r#type == "starrocks" {
        let mut host_port = payload.url.trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("mysql://").to_string();
        
        // Apply FE mapping if present
        if let Some(mapping) = &payload.fe_mapping {
            if let Some(mapped_val) = mapping.get(&host_port).and_then(|v| v.as_str()) {
                host_port = mapped_val.to_string();
            } else if !host_port.contains(':') {
                // Try with default port
                let key = format!("{}:9030", host_port);
                if let Some(mapped_val) = mapping.get(&key).and_then(|v| v.as_str()) {
                    host_port = mapped_val.to_string();
                }
            }
        }

        let conn_str = format!("mysql://{}:{}@{}/information_schema", 
            payload.username.as_deref().unwrap_or("root"), 
            password.as_deref().unwrap_or(""),
            host_port
        );
        
        let opts = mysql_async::Opts::from_url(&conn_str)
             .map_err(|e| ApiError::validation_error(format!("Invalid connection string: {}", e)))?;
        
        let builder = mysql_async::OptsBuilder::from_opts(opts)
            .prefer_socket(false);
        
        let pool = mysql_async::Pool::new(builder);
        
        let timeout_secs = payload.connection_timeout.unwrap_or(10) as u64;
        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
            let mut conn = pool.get_conn().await
                 .map_err(|e| ApiError::validation_error(format!("Connection failed: {}", e)))?;
                 
            conn.query_drop("SELECT 1").await
                 .map_err(|e| ApiError::validation_error(format!("Query failed: {}", e)))
        }).await;

        pool.disconnect().await.ok();

        match result {
             Ok(inner_result) => match inner_result {
                 Ok(_) => Ok(Json(serde_json::json!({ "status": "success", "message": "StarRocks Connection successful" }))),
                 Err(e) => Err(e)
             },
             Err(_) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("Connection timed out ({}s)", timeout_secs) })))
        }
    } else if payload.r#type == "mysql" {
         let host_port = payload.url.trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("mysql://");
         let conn_str = format!("mysql://{}:{}@{}/information_schema", 
            payload.username.as_deref().unwrap_or("root"), 
            password.as_deref().unwrap_or(""),
            host_port
        );
        
        let opts: Result<sqlx::mysql::MySqlConnectOptions, _> = conn_str.parse();
        let timeout_secs = payload.connection_timeout.unwrap_or(10) as u64;
        match opts {
            Ok(o) => {
                match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), sqlx::MySqlPool::connect_with(o)).await {
                    Ok(pool_res) => match pool_res {
                        Ok(_) => Ok(Json(serde_json::json!({ "status": "success", "message": "Connection successful" }))),
                        Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("Connection failed: {}", e) })))
                    },
                    Err(_) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("Connection timed out ({}s)", timeout_secs) })))
                }
            },
            Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": format!("Invalid connection string: {}", e) })))
        }
    } else {
        Ok(Json(serde_json::json!({ "status": "error", "message": "Unknown data source type" })))
    }
}

#[utoipa::path(
    post,
    path = "/api/resource/datasources",
    request_body = CreateDataSourceRequest,
    responses((status = 201, body = ResourceDataSource)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn create_datasource(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDataSourceRequest>,
) -> ApiResult<Json<ResourceDataSource>> {
    let id = sqlx::query(
        "INSERT INTO resource_data_sources (name, type, url, username, password, region, fe_mapping, connection_timeout) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&payload.name)
    .bind(&payload.r#type)
    .bind(&payload.url)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(payload.region.unwrap_or_else(|| "China".to_string()))
    .bind(&payload.fe_mapping)
    .bind(payload.connection_timeout.unwrap_or(10))
    .execute(&state.db).await?
    .last_insert_id();

    let ds = sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources WHERE id = ?")
        .bind(id).fetch_one(&state.db).await?;
    Ok(Json(ds))
}

#[utoipa::path(
    put,
    path = "/api/resource/datasources/{id}",
    request_body = UpdateDataSourceRequest,
    responses((status = 200, body = ResourceDataSource)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn update_datasource(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDataSourceRequest>,
) -> ApiResult<Json<ResourceDataSource>> {
    let mut current = sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources WHERE id = ?")
        .bind(id).fetch_one(&state.db).await.map_err(|_| ApiError::not_found("Data source not found"))?;

    if let Some(v) = payload.name { current.name = v; }
    if let Some(v) = payload.r#type { current.r#type = v; }
    if let Some(v) = payload.url { current.url = v; }
    if let Some(v) = payload.username { current.username = Some(v); }
    if let Some(v) = payload.password { current.password = Some(v); }
    if let Some(v) = payload.region { current.region = Some(v); }
    if let Some(v) = payload.fe_mapping { current.fe_mapping = Some(v); }
    if let Some(v) = payload.connection_timeout { current.connection_timeout = Some(v); }

    sqlx::query(
        "UPDATE resource_data_sources SET name=?, type=?, url=?, username=?, password=?, region=?, fe_mapping=?, connection_timeout=? WHERE id=?"
    )
    .bind(&current.name)
    .bind(&current.r#type)
    .bind(&current.url)
    .bind(&current.username)
    .bind(&current.password)
    .bind(&current.region)
    .bind(&current.fe_mapping)
    .bind(current.connection_timeout.unwrap_or(10))
    .bind(id)
    .execute(&state.db).await?;

    Ok(Json(current))
}

#[utoipa::path(
    delete,
    path = "/api/resource/datasources/{id}",
    responses((status = 200)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn delete_datasource(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> ApiResult<Json<()>> {
    sqlx::query("DELETE FROM resource_data_sources WHERE id = ?").bind(id).execute(&state.db).await?;
    Ok(Json(()))
}

// --- Query ---

#[utoipa::path(
    post,
    path = "/api/resource/query",
    request_body = PromQuery,
    responses((status = 200, body = serde_json::Value)),
    security(("bearer_auth" = [])),
    tag = "Resource"
)]
pub async fn query_prometheus(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PromQuery>,
) -> ApiResult<Json<Value>> {
    
    // 1. Resolve Data Source
    let ds = if let Some(ds_id) = payload.data_source_id {
        sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources WHERE id = ?")
            .bind(ds_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Data source not found"))?
    } else {
        // Fallback: Default setting (optional) OR First Prometheus
        // Just try fetching first prometheus for now
         sqlx::query_as::<_, ResourceDataSource>("SELECT * FROM resource_data_sources WHERE type = 'prometheus' LIMIT 1")
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("No default data source configured"))?
    };

    if ds.r#type == "mysql" || ds.r#type == "starrocks" {
        let host_port = ds.url.trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("mysql://");
        let conn_str = format!("mysql://{}:{}@{}/information_schema", 
            ds.username.as_deref().unwrap_or("root"), 
            ds.password.as_deref().unwrap_or(""),
            host_port
        );

        let opts = mysql_async::Opts::from_url(&conn_str)
            .map_err(|e| ApiError::validation_error(format!("Invalid connection string: {}", e)))?;
        
        let builder = mysql_async::OptsBuilder::from_opts(opts)
            .prefer_socket(false);
        
        let pool = mysql_async::Pool::new(builder);
        
        let result: ApiResult<Vec<Value>> = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut conn = pool.get_conn().await
                .map_err(|e| ApiError::validation_error(format!("Connection failed: {}", e)))?;
                
            let rows: Vec<Value> = conn.query_map(payload.query, |row: mysql_async::Row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in row.columns_ref().iter().enumerate() {
                    let name = col.name_str().to_string();
                    let val: Value = match row.get_opt(i) {
                        Some(Ok(v)) => match v {
                            mysql_async::Value::NULL => Value::Null,
                            mysql_async::Value::Bytes(b) => Value::String(String::from_utf8_lossy(&b).to_string()),
                            mysql_async::Value::Int(i) => Value::Number(i.into()),
                            mysql_async::Value::UInt(u) => Value::Number(u.into()),
                            mysql_async::Value::Float(f) => serde_json::Number::from_f64(f as f64).map(Value::Number).unwrap_or(Value::Null),
                            mysql_async::Value::Double(d) => serde_json::Number::from_f64(d).map(Value::Number).unwrap_or(Value::Null),
                            _ => Value::String(format!("{:?}", v)),
                        },
                        _ => Value::Null,
                    };
                    obj.insert(name, val);
                }
                Value::Object(obj)
            }).await.map_err(|e| ApiError::validation_error(format!("Query failed: {}", e)))?;
            Ok(rows)
        }).await.map_err(|_| ApiError::internal_error("Query timed out"))?;

        pool.disconnect().await.ok();

        return Ok(Json(serde_json::json!({
            "status": "success",
            "data": {
                "resultType": "table",
                "result": result?
            }
        })));
    }

    // 2. Execute Prometheus Query
    let client = Client::new();
    let url = if payload.start.is_some() && payload.end.is_some() {
        format!("{}/api/v1/query_range", ds.url)
    } else {
        format!("{}/api/v1/query", ds.url)
    };

    let mut params = Vec::new();
    params.push(("query", payload.query));
    if let (Some(start), Some(end)) = (payload.start, payload.end) {
        params.push(("start", start.to_string()));
        params.push(("end", end.to_string()));
        if let Some(step) = payload.step {
            params.push(("step", step));
        } else {
            let duration = end - start;
            let step_sec = (duration / 60.0).max(15.0) as i64;
            params.push(("step", step_sec.to_string()));
        }
    }

    let mut builder = client.get(&url).query(&params);
    if let (Some(u), Some(p)) = (&ds.username, &ds.password) {
        builder = builder.basic_auth(u, Some(p));
    }

    let resp = builder.send().await
        .map_err(|e| ApiError::internal_error(format!("Request failed: {}", e)))?;

    let json: Value = resp.json().await
        .map_err(|e| ApiError::internal_error(format!("Invalid JSON: {}", e)))?;

    let status = json.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
    if status != "success" {
        return Err(ApiError::validation_error(format!("Prometheus Error: {:?}", json.get("error"))));
    }

    Ok(Json(json))
}

// Deprecated settings endpoints (can be removed or kept as dummies)
// I'll keep them returning 200 or empty for now to avoid breakages if frontend calls them before reload.
#[utoipa::path(get, path = "/api/resource/settings", responses((status = 200, body = String)))]
pub async fn get_settings() -> ApiResult<Json<String>> { Ok(Json("".to_string())) }

#[utoipa::path(put, path = "/api/resource/settings", responses((status = 200)))]
pub async fn update_settings() -> ApiResult<Json<()>> { Ok(Json(())) }
