use crate::models::asset::{
    ResourceApplyRequest, ResourceApplyResponse, ResourceAssetImport, ResourceBatchDeleteRequest,
    ResourceFilterOptions, ResourceImportRequest, ResourceListResponse, ResourceQuery,
    ResourceServiceOpRequest,
};
use crate::models::common::ApiResponse;
use crate::services::asset::AssetService;
use crate::{
    AppState,
    utils::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

#[utoipa::path(
    post,
    path = "/api/asset/service-op",
    request_body = ResourceServiceOpRequest,
    responses((status = 200, body = String)),
    tag = "Asset Inventory"
)]
pub async fn service_operation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResourceServiceOpRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let service = AssetService::new(state.db.clone());
    let response = service
        .service_operation(req)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(response)))
}

#[utoipa::path(
    get,
    path = "/api/asset/filter-options",
    responses((status = 200, body = ResourceFilterOptions)),
    tag = "Asset Inventory"
)]
pub async fn get_filter_options(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<ResourceFilterOptions>>> {
    let service = AssetService::new(state.db.clone());
    let options = service
        .get_filter_options()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(options)))
}

#[utoipa::path(
    get,
    path = "/api/asset/resources",
    params(ResourceQuery),
    responses((status = 200, body = ResourceListResponse)),
    tag = "Asset Inventory"
)]
pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ResourceQuery>,
) -> ApiResult<Json<ApiResponse<ResourceListResponse>>> {
    let service = AssetService::new(state.db.clone());
    let (list, total) = service
        .list_resources(params)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(ResourceListResponse { list, total })))
}

#[utoipa::path(
    post,
    path = "/api/asset/import",
    request_body = ResourceImportRequest,
    responses((status = 200, body = usize)),
    tag = "Asset Inventory"
)]
pub async fn import_resources(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResourceImportRequest>,
) -> ApiResult<Json<ApiResponse<usize>>> {
    let service = AssetService::new(state.db.clone());
    let count = service
        .import_resources(req.items)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(count)))
}

#[utoipa::path(
    put,
    path = "/api/asset/resources",
    request_body = ResourceAssetImport,
    responses((status = 200, body = ())),
    tag = "Asset Inventory"
)]
pub async fn update_resource(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResourceAssetImport>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let service = AssetService::new(state.db.clone());
    service
        .update_resource(req)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/api/asset/resources/batch-delete",
    request_body = ResourceBatchDeleteRequest,
    responses((status = 200, body = ())),
    tag = "Asset Inventory"
)]
pub async fn delete_resources(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResourceBatchDeleteRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let service = AssetService::new(state.db.clone());
    service
        .delete_resources(req.private_ips)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/api/asset/apply",
    request_body = ResourceApplyRequest,
    responses((status = 200, body = ResourceApplyResponse)),
    tag = "Asset Inventory"
)]
pub async fn apply_resources(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResourceApplyRequest>,
) -> ApiResult<Json<ApiResponse<ResourceApplyResponse>>> {
    let service = AssetService::new(state.db.clone());
    let response = service
        .apply_resources(req)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(response)))
}
