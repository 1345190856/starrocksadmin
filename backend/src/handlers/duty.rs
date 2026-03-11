use crate::{
    AppState,
    models::duty::*,
    utils::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Duration as ChronoDuration;
use std::sync::Arc;

// --- Personnel Handlers ---

/// List all duty personnel
#[utoipa::path(
    get,
    path = "/api/duty/personnel",
    responses(
        (status = 200, description = "List duty personnel", body = Vec<DutyPersonnel>)
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn list_personnel(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<DutyPersonnel>>> {
    let personnel =
        sqlx::query_as::<_, DutyPersonnel>("SELECT * FROM duty_personnel ORDER BY name")
            .fetch_all(&state.db)
            .await?;

    Ok(Json(personnel))
}

/// Create new duty personnel
#[utoipa::path(
    post,
    path = "/api/duty/personnel",
    request_body = CreateDutyPersonnelRequest,
    responses(
        (status = 201, description = "Personnel created", body = DutyPersonnel)
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn create_personnel(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDutyPersonnelRequest>,
) -> ApiResult<Json<DutyPersonnel>> {
    let id = sqlx::query(
        "INSERT INTO duty_personnel (name, org_lvl1, org_lvl2, email, phone, duty_platform, responsible_domain, countries) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&payload.name)
    .bind(&payload.org_lvl1)
    .bind(&payload.org_lvl2)
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(&payload.duty_platform)
    .bind(&payload.responsible_domain)
    .bind(&payload.countries)
    .execute(&state.db)
    .await?
    .last_insert_id();

    let personnel = sqlx::query_as::<_, DutyPersonnel>("SELECT * FROM duty_personnel WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(personnel))
}

/// Update duty personnel
#[utoipa::path(
    put,
    path = "/api/duty/personnel/{id}",
    request_body = UpdateDutyPersonnelRequest,
    responses(
        (status = 200, description = "Personnel updated", body = DutyPersonnel)
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn update_personnel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDutyPersonnelRequest>,
) -> ApiResult<Json<DutyPersonnel>> {
    let mut current =
        sqlx::query_as::<_, DutyPersonnel>("SELECT * FROM duty_personnel WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| ApiError::not_found("Personnel not found"))?;

    if let Some(name) = payload.name {
        current.name = name;
    }
    if let Some(org_lvl1) = payload.org_lvl1 {
        current.org_lvl1 = Some(org_lvl1);
    }
    if let Some(org_lvl2) = payload.org_lvl2 {
        current.org_lvl2 = Some(org_lvl2);
    }
    if let Some(email) = payload.email {
        current.email = email;
    }
    if let Some(phone) = payload.phone {
        current.phone = phone;
    }
    if let Some(duty_platform) = payload.duty_platform {
        current.duty_platform = Some(duty_platform);
    }
    if let Some(responsible_domain) = payload.responsible_domain {
        current.responsible_domain = Some(responsible_domain);
    }
    if let Some(countries) = payload.countries {
        current.countries = Some(countries);
    }

    sqlx::query(
        "UPDATE duty_personnel SET name=?, org_lvl1=?, org_lvl2=?, email=?, phone=?, duty_platform=?, responsible_domain=?, countries=? WHERE id=?"
    )
    .bind(&current.name)
    .bind(&current.org_lvl1)
    .bind(&current.org_lvl2)
    .bind(&current.email)
    .bind(&current.phone)
    .bind(&current.duty_platform)
    .bind(&current.responsible_domain)
    .bind(&current.countries)
    .bind(id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_as::<_, DutyPersonnel>("SELECT * FROM duty_personnel WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(updated))
}

/// Delete duty personnel
#[utoipa::path(
    delete,
    path = "/api/duty/personnel/{id}",
    responses(
        (status = 200, description = "Personnel deleted")
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn delete_personnel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<()>> {
    sqlx::query("DELETE FROM duty_personnel WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(()))
}

// --- Schedule Handlers ---

/// Get duty schedule
#[utoipa::path(
    get,
    path = "/api/duty/schedule",
    params(
        DutyScheduleQuery
    ),
    responses(
        (status = 200, description = "List schedule", body = Vec<DutySchedule>)
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DutyScheduleQuery>,
) -> ApiResult<Json<Vec<DutySchedule>>> {
    let mut query = String::from(
        r#"
        SELECT s.*, p.name as personnel_name, p.email as personnel_email, p.duty_platform 
        FROM duty_schedule s
        JOIN duty_personnel p ON s.personnel_id = p.id
        WHERE 1=1
        "#,
    );

    let mut args = sqlx::mysql::MySqlArguments::default();
    use sqlx::Arguments;

    if let Some(start) = params.start_date {
        query.push_str(" AND s.duty_date >= ?");
        args.add(start);
    }
    if let Some(end) = params.end_date {
        query.push_str(" AND s.duty_date <= ?");
        args.add(end);
    }
    if let Some(country) = params.country {
        query.push_str(" AND s.country = ?");
        args.add(country);
    }
    if let Some(platform) = params.duty_platform {
        query.push_str(" AND (s.duty_platform = ? OR p.duty_platform = ?)");
        args.add(&platform);
        args.add(&platform);
    }

    query.push_str(" ORDER BY s.duty_date ASC, s.country ASC");

    let schedule = sqlx::query_as_with::<_, DutySchedule, _>(&query, args)
        .fetch_all(&state.db)
        .await?;

    Ok(Json(schedule))
}

/// Batch assign/update schedule
#[utoipa::path(
    post,
    path = "/api/duty/schedule/batch",
    request_body = BatchAssignDutyRequest,
    responses(
        (status = 200, description = "Schedule updated")
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn batch_assign_schedule(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchAssignDutyRequest>,
) -> ApiResult<Json<()>> {
    let mut tx = state.db.begin().await?;

    for item in payload.schedules {
        // Delete existing for this slot
        sqlx::query(
            "DELETE FROM duty_schedule WHERE duty_date = ? AND country = ? AND shift_type = ?",
        )
        .bind(item.duty_date)
        .bind(&item.country)
        .bind(&item.shift_type)
        .execute(&mut *tx)
        .await?;

        // Insert new for each personnel
        for pid in item.personnel_ids {
            sqlx::query(
                "INSERT INTO duty_schedule (duty_date, country, shift_type, personnel_id) VALUES (?, ?, ?, ?)"
            )
            .bind(item.duty_date)
            .bind(&item.country)
            .bind(&item.shift_type)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                 // Check foreign key error?
                 tracing::error!("Failed to assign duty: {}", e);
                 ApiError::validation_error("Invalid personnel ID or database error")
            })?;
        }
    }

    tx.commit().await?;

    Ok(Json(()))
}

/// Delete duty schedule
#[utoipa::path(
    delete,
    path = "/api/duty/schedule/{id}",
    responses(
        (status = 200, description = "Schedule deleted")
    ),
    security(("bearer_auth" = [])),
    tag = "Duty"
)]
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<()>> {
    sqlx::query("DELETE FROM duty_schedule WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(()))
}

// --- Rotation Handlers ---

#[derive(serde::Deserialize)]
pub struct NotifyManualRequest {
    pub bot_ids: Vec<String>,
}

pub async fn notify_manual(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NotifyManualRequest>,
) -> ApiResult<Json<()>> {
    let message = state
        .duty_service
        .get_manual_notification_message()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to build message: {}", e)))?;
    let mentions = state
        .duty_service
        .get_manual_notification_mentions()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to collect mentions: {}", e)))?;

    for bot_id in payload.bot_ids {
        let _ = state
            .alert_service
            .send_notification(&bot_id, &message, mentions.clone())
            .await;
    }

    Ok(Json(()))
}

pub async fn list_rotations(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<DutyRotation>>> {
    // 每次进入页面时，主动检查并转动过期的轮换，保证前台卡片展示准确
    let _ = state
        .duty_service
        .check_and_rotate_expired_rotations()
        .await;

    let rotations =
        sqlx::query_as::<_, DutyRotation>("SELECT * FROM duty_rotation ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    Ok(Json(rotations))
}

pub async fn save_rotation(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDutyRotationRequest>,
) -> ApiResult<Json<DutyRotation>> {
    let personnel_ids_json =
        serde_json::to_string(&payload.personnel_ids).unwrap_or_else(|_| "[]".to_string());

    let existing: Option<DutyRotation> =
        sqlx::query_as("SELECT * FROM duty_rotation WHERE name = ?")
            .bind(&payload.name)
            .fetch_optional(&state.db)
            .await?;

    let bot_ids = payload
        .bot_ids
        .or(existing.as_ref().and_then(|e| e.bot_ids.clone()));
    let auto_notify = payload.auto_notify.unwrap_or_else(|| {
        existing
            .as_ref()
            .and_then(|e| e.auto_notify)
            .unwrap_or(false)
    });
    let notify_advance_hours = payload.notify_advance_hours.unwrap_or_else(|| {
        existing
            .as_ref()
            .and_then(|e| e.notify_advance_hours)
            .unwrap_or(7)
    });

    let mut tx = state.db.begin().await?;

    // Delete existing rotation with same name
    sqlx::query("DELETE FROM duty_rotation WHERE name = ?")
        .bind(&payload.name)
        .execute(&mut *tx)
        .await?;

    let last_notified_date = existing.as_ref().and_then(|e| e.last_notified_date);

    let new_start = payload.start_date;
    let total_days = payload.period_days;
    let new_end = new_start + ChronoDuration::days((total_days - 1) as i64);

    let id = sqlx::query(
        "INSERT INTO duty_rotation (name, personnel_ids, start_date, end_date, period_days, country, bot_ids, auto_notify, notify_advance_hours, last_notified_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&payload.name)
    .bind(&personnel_ids_json)
    .bind(new_start)
    .bind(new_end)
    .bind(payload.period_days)
    .bind(&payload.country)
    .bind(&bot_ids)
    .bind(auto_notify)
    .bind(notify_advance_hours)
    .bind(last_notified_date)
    .execute(&mut *tx)
    .await?
    .last_insert_id();

    // Generate schedules
    if !payload.personnel_ids.is_empty() {
        // Clear ALL existing schedules for this platform and country to prevent "ghost" schedules
        // that were outside the current new range but existed previously.
        sqlx::query("DELETE FROM duty_schedule WHERE duty_platform = ? AND country = ?")
            .bind(&payload.name)
            .bind(&payload.country)
            .execute(&mut *tx)
            .await?;

        let mut current_date = new_start;
        let mut personnel_idx = 0;

        while current_date <= new_end {
            let pid = payload.personnel_ids[personnel_idx % payload.personnel_ids.len()];

            // For each day in the period
            for _ in 0..payload.period_days {
                if current_date > new_end {
                    break;
                }

                // Insert new schedule with platform
                sqlx::query("INSERT INTO duty_schedule (duty_date, country, duty_platform, shift_type, personnel_id) VALUES (?, ?, ?, 'All Day', ?)")
                    .bind(current_date)
                    .bind(&payload.country)
                    .bind(&payload.name)
                    .bind(pid)
                    .execute(&mut *tx)
                    .await?;

                current_date += ChronoDuration::days(1);
            }
            personnel_idx += 1;
        }
    }

    tx.commit().await?;

    let rotation = sqlx::query_as::<_, DutyRotation>("SELECT * FROM duty_rotation WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(rotation))
}

pub async fn update_rotation_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRotationConfigRequest>,
) -> ApiResult<Json<()>> {
    sqlx::query("UPDATE duty_rotation SET bot_ids = ?, auto_notify = ?, notify_advance_hours = ? WHERE name = ?")
        .bind(&payload.bot_ids)
        .bind(payload.auto_notify)
        .bind(payload.notify_advance_hours)
        .bind(&payload.name)
        .execute(&state.db)
        .await?;

    Ok(Json(()))
}
