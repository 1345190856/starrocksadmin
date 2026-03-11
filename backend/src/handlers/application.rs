use std::sync::Arc;
use axum::{Json, extract::{Path, State}};
use crate::{AppState, models::application::*, utils::{ApiResult}};

#[utoipa::path(
    get,
    path = "/api/applications",
    responses((status = 200, body = Vec<Application>)),
    security(("bearer_auth" = [])),
    tag = "Application"
)]
pub async fn list_applications(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Application>>> {
    let apps = state.application_service.list_applications().await?;
    Ok(Json(apps))
}

#[utoipa::path(
    post,
    path = "/api/applications",
    request_body = CreateApplicationRequest,
    responses((status = 201, body = Application)),
    security(("bearer_auth" = [])),
    tag = "Application"
)]
pub async fn create_application(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateApplicationRequest>,
) -> ApiResult<Json<Application>> {
    let app = state.application_service.create_application(payload).await?;
    Ok(Json(app))
}

#[utoipa::path(
    put,
    path = "/api/applications/{id}",
    request_body = UpdateApplicationRequest,
    responses((status = 200, body = Application)),
    security(("bearer_auth" = [])),
    tag = "Application"
)]
pub async fn update_application(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateApplicationRequest>,
) -> ApiResult<Json<Application>> {
    let app = state.application_service.update_application(id, payload).await?;
    Ok(Json(app))
}

#[utoipa::path(
    delete,
    path = "/api/applications/{id}",
    responses((status = 200)),
    security(("bearer_auth" = [])),
    tag = "Application"
)]
pub async fn delete_application(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<()>> {
    state.application_service.delete_application(id).await?;
    Ok(Json(()))
}
