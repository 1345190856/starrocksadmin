use crate::models::common::ApiResponse;
use crate::models::data_sync::{ProxyWebhookRequest, SyncSubmitRequest};
use crate::services::data_sync::DataSyncService;
use crate::services::system_service::SystemService;
use crate::{
    AppState,
    utils::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
};
use base64::{Engine as _, engine::general_purpose};
use rsa::{Oaep, RsaPublicKey, pkcs8::DecodePublicKey};
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;

const N8N_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEA4WXootKdshJpeR5hfyYo
xgYsKuv1JDJQUvV6P0U0iGbCvPfx2U3LtJhPk4DrAtWwV7jl6L7TZLlLw70WKtx1
4NBEWd5eaxGxrkDbiX6KywpPujjB9xGrUxqvuzoOAx5Nsa7zpQ+E5GUjqA6lvp80
+TrflUsv4+pReq7KDpdm8mdj3zy8WFF/nWNUYpjgxe3M8mqRDAeFr4UA6ZhWPs9U
BmtkwYIKyBzvvlr8053vcqkqx3x44V6noqsm093Ivexs4dnjCl5xsRnsT9eUV1uW
w4/Ts5xvBqv3aHQbzxaZYtTAkR45Poj2wksluHzVOl9aQTcXZsBTueMQww4hVq3K
a6YOkgt/WDEapwArtua0HDnhpLg+F++4ixScQ5315W1LhLmtgaa6DUdyVBEA/dGs
3STSBCz0DL9yV3YP+NnchvU5VYca5ThQCiHdNUpPxKiav8pKaXydHUFeWmOuV+BM
Vbh2SYap3uYmXAJxDOF00j0DCKlNYZet7unxkm43NPXhbnF9fO/E7H9WZXf6NQZ3
qcjjI3kIZnWab+wd079C0Z5FWwX+fUQhhnI0sku2jPy8myCQBf/ighFq9lgGij+l
wDlCyH99DJsvINegD/wBteDV63FC62rSRhRZuouhd8Bd5lEQhWkrAAk8IXF0nh0C
F98xUGepEi+PyUK84FQnJfMCAwEAAQ==
-----END PUBLIC KEY-----"#;

#[utoipa::path(
    post,
    path = "/api/data-sync/submit",
    request_body = SyncSubmitRequest,
    responses((status = 200, body = ApiResponse<()>)),
    tag = "Data Sync"
)]
pub async fn submit_sync_ticket(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user_id): axum::extract::Extension<i64>,
    Json(req): Json<SyncSubmitRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user = state
        .auth_service
        .get_user_by_id(user_id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let service = DataSyncService::new(state.db.clone());
    service
        .submit_ticket(req.clone(), &user.username)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Trigger notification webhook
    let notification_url = "http://10.20.47.19:5678/webhook/73c5d0c9-000a-4a84-9694-60e6d7f6c2eb";
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let tables_info = req.selected_tables.as_array().cloned().unwrap_or_default();
    let db_names: Vec<String> = tables_info
        .iter()
        .filter_map(|t| {
            t.get("dbName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    let mut unique_dbs = db_names.clone();
    unique_dbs.sort();
    unique_dbs.dedup();
    let dbs = unique_dbs.join(", ");

    let table_names: Vec<String> = tables_info
        .iter()
        .filter_map(|t| {
            t.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    let tables = table_names.join(", ");

    let notification_payload = json!({
        "country": req.country,
        "source_ip": req.source_ip,
        "db": dbs,
        "table": tables,
        "remark": req.remark.unwrap_or_default(),
        "type": "数据同步工单"
    });

    let _ = client
        .post(notification_url)
        .json(&notification_payload)
        .send()
        .await;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/api/data-sync/proxy-webhook",
    request_body = ProxyWebhookRequest,
    responses((status = 200, body = serde_json::Value)),
    tag = "Data Sync"
)]
pub async fn proxy_webhook(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProxyWebhookRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = DataSyncService::new(state.db.clone());
    let secret = service
        .get_secret(&req.ip, &req.port, &req.country)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("请联系管理员新增数据源".to_string()))?;

    let system_service = SystemService::new(state.db.clone());
    let webhook_url = system_service
        .get_config("data_sync_webhook_url")
        .await?
        .map(|c| c.config_value)
        .unwrap_or_else(|| {
            "https://example.com/webhook/d6db9559-b0ce-42db-b472-3b803579cf19".to_string()
        });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Reconstruct the payload based on what the original webhook expects
    let mut final_payload = json!({
        "country": req.country,
        "username": secret.username,
        "password": secret.password,
        "command": req.command,
    });

    if let Some(t) = req.r#type {
        final_payload["type"] = json!(t);
    }

    if let Some(db) = req.db {
        final_payload["db"] = json!(db);
    }
    final_payload["ip"] = json!(req.ip);
    final_payload["port"] = json!(req.port);

    // Encrypt the payload for n8n
    let json_str = serde_json::to_string(&final_payload).map_err(|e| {
        ApiError::internal_error(format!("Failed to serialize payload for encryption: {}", e))
    })?;

    let pub_key = RsaPublicKey::from_public_key_pem(N8N_PUBLIC_KEY)
        .map_err(|e| ApiError::internal_error(format!("Failed to parse n8n public key: {}", e)))?;

    let encrypted_base64 = {
        let mut rng = rand::thread_rng();
        let padding = Oaep::new::<Sha256>();
        let enc_data = pub_key
            .encrypt(&mut rng, padding, json_str.as_bytes())
            .map_err(|e| ApiError::internal_error(format!("Encryption failed: {}", e)))?;
        general_purpose::STANDARD.encode(enc_data)
    };
    let wrapper = json!({ "data": encrypted_base64 });

    let res = client
        .post(&webhook_url)
        .json(&wrapper)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let status = res.status();
    let body_text = res
        .text()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let body: serde_json::Value = if body_text.is_empty() {
        json!({"stdout": "", "stderr": ""})
    } else {
        serde_json::from_str(&body_text).map_err(|e| {
            ApiError::internal_error(format!(
                "Failed to parse webhook JSON response: {}. Body: {}",
                e, body_text
            ))
        })?
    };

    if !status.is_success() {
        return Err(ApiError::internal_error(format!("Webhook returned error: {}", body)));
    }

    Ok(Json(body))
}

#[utoipa::path(
    get,
    path = "/api/data-sync/list",
    responses((status = 200, body = ApiResponse<Vec<SyncList>>)),
    tag = "Data Sync"
)]
pub async fn list_tickets(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<Vec<crate::models::data_sync::SyncList>>>> {
    let service = DataSyncService::new(state.db.clone());
    let list = service
        .list_tickets()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(list)))
}

pub async fn update_processor(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user_id): axum::extract::Extension<i64>,
    Path(id): Path<i32>,
    Json(payload): Json<crate::models::data_sync::UpdateProcessorRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Check if user has 'admin' or 'sre' role
    let role_codes = state.auth_service.get_user_role_codes(user_id).await?;
    if !role_codes.contains(&"admin".to_string()) && !role_codes.contains(&"sre".to_string()) {
        return Err(ApiError::unauthorized("Only admin and sre roles can update sync tickets"));
    }

    let service = DataSyncService::new(state.db.clone());
    service
        .update_processor(id, &payload.processor)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn update_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user_id): axum::extract::Extension<i64>,
    Path(id): Path<i32>,
    Json(payload): Json<crate::models::data_sync::UpdateStatusRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Check if user has 'admin' or 'sre' role
    let role_codes = state.auth_service.get_user_role_codes(user_id).await?;
    if !role_codes.contains(&"admin".to_string()) && !role_codes.contains(&"sre".to_string()) {
        return Err(ApiError::unauthorized("Only admin and sre roles can update sync tickets"));
    }

    let service = DataSyncService::new(state.db.clone());
    service
        .update_status(id, &payload.approval_status)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn approve_sync_ticket(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user_id): axum::extract::Extension<i64>,
    Path(id): Path<i32>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Check if user has 'admin' or 'sre' role
    let role_codes = state.auth_service.get_user_role_codes(user_id).await?;
    if !role_codes.contains(&"admin".to_string()) && !role_codes.contains(&"sre".to_string()) {
        return Err(ApiError::unauthorized("Only admin and sre roles can approve sync tickets"));
    }

    let service = DataSyncService::new(state.db.clone());
    service
        .approve_ticket(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}
