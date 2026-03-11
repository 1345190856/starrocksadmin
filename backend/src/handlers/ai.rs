use axum::{
    extract::{Path, State},
    Json, Extension,
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use crate::AppState;
use crate::middleware::auth::OrgContext;
#[allow(unused_imports)]
use crate::models::ai::{CreateAiSettingRequest, UpdateAiSettingRequest, AiSetting};
#[allow(unused_imports)]
use crate::utils::{ApiError, ApiResult};

#[utoipa::path(
    get,
    path = "/api/ai/settings",
    responses(
        (status = 200, description = "List all AI settings", body = [AiSetting]),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_ai_settings(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<OrgContext>,
) -> ApiResult<impl IntoResponse> {
    let settings = state.ai_service
        .list_settings(&org_ctx.username, org_ctx.is_super_admin)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(settings)))
}

#[utoipa::path(
    post,
    path = "/api/ai/settings",
    request_body = CreateAiSettingRequest,
    responses(
        (status = 201, description = "Create a new AI setting", body = AiSetting),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_ai_setting(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<OrgContext>,
    Json(req): Json<CreateAiSettingRequest>,
) -> ApiResult<impl IntoResponse> {
    let setting = state.ai_service
        .create_setting(req, &org_ctx.username)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(setting)))
}

#[utoipa::path(
    put,
    path = "/api/ai/settings/{id}",
    request_body = UpdateAiSettingRequest,
    params(
        ("id" = i32, Path, description = "AI setting ID")
    ),
    responses(
        (status = 200, description = "Update AI setting", body = AiSetting),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_ai_setting(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<OrgContext>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateAiSettingRequest>,
) -> ApiResult<impl IntoResponse> {
    let setting = state.ai_service
        .update_setting(id, req, &org_ctx.username, org_ctx.is_super_admin)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(setting)))
}

#[utoipa::path(
    delete,
    path = "/api/ai/settings/{id}",
    params(
        ("id" = i32, Path, description = "AI setting ID")
    ),
    responses(
        (status = 204, description = "Delete AI setting"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_ai_setting(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<OrgContext>,
    Path(id): Path<i32>,
) -> ApiResult<impl IntoResponse> {
    state.ai_service
        .delete_setting(id, &org_ctx.username, org_ctx.is_super_admin)
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
