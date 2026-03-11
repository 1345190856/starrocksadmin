use axum::{Json, extract::State};
use std::sync::Arc;

use crate::AppState;
use crate::models::RuntimeInfo;
use crate::services::StarRocksClient;
use crate::utils::ApiResult;

// Get runtime info for a cluster
#[utoipa::path(
    get,
    path = "/api/clusters/system/runtime_info",
    responses(
        (status = 200, description = "Runtime information", body = RuntimeInfo),
        (status = 404, description = "No active cluster found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "System"
)]
pub async fn get_runtime_info(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(org_ctx): axum::extract::Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<RuntimeInfo>> {
    // Get the active cluster with organization isolation
    let cluster = if org_ctx.is_super_admin {
        state.cluster_service.get_active_cluster().await?
    } else {
        state
            .cluster_service
            .get_active_cluster_by_org(org_ctx.organization_id)
            .await?
    };
    let client = StarRocksClient::new(cluster, state.mysql_pool_manager.clone());
    let runtime_info = client.get_runtime_info().await?;
    Ok(Json(runtime_info))
}

#[utoipa::path(
    get,
    path = "/api/system/config/{key}",
    tag = "System",
    responses(
        (status = 200, description = "Get system config", body = Option<SystemConfig>)
    )
)]
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> ApiResult<Json<Option<crate::models::system::SystemConfig>>> {
    let config = state.system_service.get_config(&key).await?;
    Ok(Json(config))
}

#[utoipa::path(
    put,
    path = "/api/system/config/{key}",
    tag = "System",
    request_body = UpdateSystemConfigRequest,
    responses(
        (status = 200, description = "Update system config")
    )
)]
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(key): axum::extract::Path<String>,
    Json(req): Json<crate::models::system::UpdateSystemConfigRequest>,
) -> ApiResult<()> {
    state.system_service.update_config(&key, req).await?;
    Ok(())
}
