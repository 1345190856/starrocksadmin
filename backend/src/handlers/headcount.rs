use crate::models::common::ApiResponse;
use crate::models::headcount::{EmployeeListResponse, EmployeeQuery};
use crate::services::headcount::HeadcountService;
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
    get,
    path = "/api/headcount/employees",
    params(EmployeeQuery),
    responses((status = 200, body = EmployeeListResponse)),
    tag = "Headcount"
)]
pub async fn list_employees(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EmployeeQuery>,
) -> ApiResult<Json<ApiResponse<EmployeeListResponse>>> {
    let service = HeadcountService::new(state.db.clone());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let (list, total) = service
        .list_employees(page, page_size, params.query, params.sort_by, params.sort_order)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(EmployeeListResponse { list, total })))
}

#[utoipa::path(
    post,
    path = "/api/headcount/sync",
    responses((status = 200, body = String)),
    tag = "Headcount"
)]
pub async fn sync_employees(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let service = HeadcountService::new(state.db.clone());
    let count = service
        .sync_employees()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(format!("Synced {} employees", count))))
}
