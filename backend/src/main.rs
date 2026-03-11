use axum::{
    Router,
    body::Body,
    http::{HeaderValue, StatusCode, Uri, header},
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod config;
mod db;
mod embedded;
mod handlers;
mod middleware;
mod models;
mod services;
mod utils;

// Include tests module only in test builds
#[cfg(test)]
mod tests;

use config::Config;
use embedded::WebAssets;
use services::{
    AiService, AlertService, ApplicationService, AssetService, AuthService, CasbinService,
    ClusterService, DataStatisticsService, DutyService, MetricsCollectorService, MySQLPoolManager,
    OrganizationService, OverviewService, PermissionService, RoleService, SystemFunctionService,
    SystemService, UserRoleService, UserService,
};
use sqlx::MySqlPool;
use utils::{JwtUtil, ScheduledExecutor};

/// Application shared state
///
/// Design Philosophy: Keep it simple - Rust's type system IS our DI container.
/// No need for Service Container pattern with dyn Any.
/// All services are wrapped in Arc for cheap cloning and thread safety.
#[derive(Clone)]
pub struct AppState {
    // Core dependencies
    pub db: MySqlPool,

    // Managers
    pub mysql_pool_manager: Arc<MySQLPoolManager>,
    pub jwt_util: Arc<JwtUtil>,

    // Config
    pub audit_config: config::AuditLogConfig,

    // Services (grouped by domain)
    pub auth_service: Arc<AuthService>,
    pub cluster_service: Arc<ClusterService>,
    pub organization_service: Arc<OrganizationService>,
    pub system_function_service: Arc<SystemFunctionService>,
    pub metrics_collector_service: Arc<MetricsCollectorService>,
    pub data_statistics_service: Arc<DataStatisticsService>,
    pub overview_service: Arc<OverviewService>,

    // RBAC Services
    pub casbin_service: Arc<CasbinService>,
    pub permission_service: Arc<PermissionService>,
    pub role_service: Arc<RoleService>,
    pub user_role_service: Arc<UserRoleService>,
    pub user_service: Arc<UserService>,
    pub alert_service: Arc<AlertService>,
    pub application_service: Arc<ApplicationService>,
    pub ai_service: Arc<AiService>,
    pub asset_service: Arc<AssetService>,
    pub duty_service: Arc<DutyService>,
    pub system_service: Arc<SystemService>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::get_me,
        handlers::auth::update_me,
        handlers::cluster::create_cluster,
        handlers::cluster::list_clusters,
        handlers::cluster::get_active_cluster,
        handlers::cluster::get_cluster,
        handlers::cluster::update_cluster,
        handlers::cluster::delete_cluster,
        handlers::cluster::activate_cluster,
        handlers::organization::create_organization,
        handlers::organization::list_organizations,
        handlers::organization::get_organization,
        handlers::organization::update_organization,
        handlers::organization::delete_organization,
        handlers::cluster::get_cluster_health,
        handlers::backend::list_backends,
        handlers::frontend::list_frontends,
        handlers::materialized_view::list_materialized_views,
        handlers::materialized_view::get_materialized_view,
        handlers::materialized_view::get_materialized_view_ddl,
        handlers::materialized_view::create_materialized_view,
        handlers::materialized_view::delete_materialized_view,
        handlers::materialized_view::refresh_materialized_view,
        handlers::materialized_view::cancel_refresh_materialized_view,
        handlers::materialized_view::alter_materialized_view,
        handlers::query::list_catalogs,
        handlers::query::list_databases,
        handlers::query::list_catalogs_with_databases,
        handlers::query::list_queries,
        handlers::query::kill_query,
        handlers::query::execute_sql,
        handlers::query_history::list_query_history,
        handlers::sessions::get_sessions,
        handlers::sessions::kill_session,
        handlers::variables::get_variables,
        handlers::variables::update_variable,
        handlers::profile::list_profiles,
        handlers::profile::get_profile,
        handlers::profile::analyze_profile_handler,
        handlers::system_management::get_system_functions,
        handlers::system_management::get_system_function_detail,
        handlers::system::get_runtime_info,
        handlers::overview::get_cluster_overview,
        handlers::overview::get_health_cards,
        handlers::overview::get_performance_trends,
        handlers::overview::get_resource_trends,
        handlers::overview::get_data_statistics,
        handlers::overview::get_capacity_prediction,
        handlers::overview::get_extended_cluster_overview,
        handlers::cluster::test_cluster_connection,
        // RBAC Handlers
        handlers::role::list_roles,
        handlers::role::get_role,
        handlers::role::create_role,
        handlers::role::update_role,
        handlers::role::delete_role,
        handlers::role::get_role_with_permissions,
        handlers::role::update_role_permissions,
        handlers::permission::list_permissions,
        handlers::permission::list_menu_permissions,
        handlers::permission::list_api_permissions,
        handlers::permission::get_permission_tree,
        handlers::permission::get_current_user_permissions,
        handlers::user_role::get_user_roles,
        handlers::user_role::assign_role_to_user,
        handlers::user_role::remove_role_from_user,
        handlers::user::list_users,
        handlers::user::get_user,
        handlers::user::create_user,
        handlers::user::update_user,
        handlers::user::delete_user,
        // Duty Handlers
        handlers::duty::list_personnel,
        handlers::duty::create_personnel,
        handlers::duty::update_personnel,
        handlers::duty::delete_personnel,
        handlers::duty::get_schedule,
        handlers::duty::batch_assign_schedule,
        // Headcount
        handlers::headcount::list_employees,
        handlers::headcount::sync_employees,
        handlers::application::list_applications,
        handlers::application::create_application,
        handlers::application::update_application,
        handlers::application::delete_application,
        handlers::ai::list_ai_settings,
        handlers::ai::create_ai_setting,
        handlers::ai::update_ai_setting,
        handlers::ai::delete_ai_setting,
        handlers::asset::list_resources,
        handlers::asset::import_resources,
        handlers::asset::update_resource,
        handlers::asset::get_filter_options,
        handlers::asset::delete_resources,
        handlers::asset::apply_resources,
    ),
    components(
        schemas(
            models::User,
            models::UserResponse,
            models::UserWithRolesResponse,
            models::CreateUserRequest,
            models::AdminCreateUserRequest,
            models::LoginRequest,
            models::LoginResponse,
            models::AdminUpdateUserRequest,
            models::Cluster,
            models::ClusterResponse,
            models::CreateClusterRequest,
            models::UpdateClusterRequest,
            models::ClusterHealth,
            models::HealthStatus,
            models::HealthCheck,
            models::Backend,
            models::Frontend,
            models::MaterializedView,
            models::CreateMaterializedViewRequest,
            models::RefreshMaterializedViewRequest,
            models::AlterMaterializedViewRequest,
            models::MaterializedViewDDL,
            models::Query,
            models::QueryExecuteRequest,
            models::QueryExecuteResponse,
            models::CatalogWithDatabases,
            models::CatalogsWithDatabasesResponse,
            models::QueryHistoryItem,
            models::QueryHistoryResponse,
            models::ProfileListItem,
            models::ProfileDetail,
            models::RuntimeInfo,
            models::MetricsSummary,
            models::SystemFunction,
            models::CreateFunctionRequest,
            models::UpdateOrderRequest,
            models::FunctionOrder,
            models::Role,
            models::RoleResponse,
            models::CreateRoleRequest,
            models::UpdateRoleRequest,
            models::RoleWithPermissions,
            models::Permission,
            models::PermissionResponse,
            models::PermissionTree,
            models::UpdateRolePermissionsRequest,
            models::AssignUserRoleRequest,
            services::ClusterOverview,
            services::ExtendedClusterOverview,
            services::HealthCard,
            services::HealthStatus,
            models::DutyPersonnel,
            models::CreateDutyPersonnelRequest,
            models::UpdateDutyPersonnelRequest,
            models::DutySchedule,
            models::BatchAssignDutyRequest,
            models::headcount::Employee,
            models::headcount::EmployeeListResponse,
            models::headcount::EmployeeQuery,
            models::asset::ResourceFilterOptions,
            services::ClusterHealth,
            services::KeyPerformanceIndicators,
            services::ResourceMetrics,
            services::MaterializedViewStats,
            services::LoadJobStats,
            services::TransactionStats,
            services::SchemaChangeStats,
            services::CompactionStats,
            services::BECompactionScore,
            services::CompactionDetailStats,
            services::TopPartitionByScore,
            services::CompactionTaskStats,
            services::CompactionDurationStats,
            services::SessionStats,
            services::RunningQuery,
            services::NetworkIOStats,
            services::Alert,
            services::AlertLevel,
            services::PerformanceTrends,
            services::ResourceTrends,
            services::MetricsSnapshot,
            services::DataStatistics,
            services::TopTableBySize,
            services::TopTableByAccess,
            services::CapacityPrediction,
            models::Application,
            models::CreateApplicationRequest,
            models::UpdateApplicationRequest,
            models::AiSetting,
            models::CreateAiSettingRequest,
            models::UpdateAiSettingRequest,
            models::asset::ResourceApplyRequest,
            models::asset::ResourceApplyResponse,
        )
    ),
    tags(
        (name = "Authentication", description = "User authentication endpoints"),
        (name = "Clusters", description = "Cluster management endpoints"),
        (name = "Backends", description = "Backend node management"),
        (name = "Frontends", description = "Frontend node management"),
        (name = "Materialized Views", description = "Materialized view management"),
        (name = "Queries", description = "Query management"),
        (name = "Profiles", description = "Query profile management"),
        (name = "System", description = "System information"),
        (name = "Roles", description = "Role management"),
        (name = "Permissions", description = "Permission management"),
        (name = "Users", description = "User role management"),
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration first
    let config = Config::load()?;

    // Initialize logging
    let log_filter = tracing_subscriber::EnvFilter::new(&config.logging.level);

    let registry = tracing_subscriber::registry().with(log_filter);

    // Add file logging if configured
    if let Some(log_file) = &config.logging.file {
        // Ensure log directory exists
        let log_path = std::path::Path::new(log_file);
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Extract directory and filename prefix from config
        let log_dir = log_path.parent().and_then(|p| p.to_str()).unwrap_or("logs");
        let file_name = log_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("starrocks-admin.log");
        // Remove .log extension if present (rolling appender adds date suffix)
        let file_prefix = file_name.strip_suffix(".log").unwrap_or(file_name);

        let file_appender = tracing_appender::rolling::daily(log_dir, file_prefix);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // Alert specific logger
        let alert_file_appender = tracing_appender::rolling::daily(log_dir, "alert");
        let (alert_non_blocking, _alert_guard) =
            tracing_appender::non_blocking(alert_file_appender);
        let alert_layer = tracing_subscriber::fmt::layer()
            .with_writer(alert_non_blocking)
            .with_ansi(false)
            .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                metadata.target().contains("alert_service") || metadata.target().contains("alert")
            }));

        registry
            .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
            .with(alert_layer)
            .with(tracing_subscriber::fmt::layer())
            .init();

        // Keep guards alive
        Box::leak(Box::new(_guard));
        Box::leak(Box::new(_alert_guard));
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
    tracing::info!("StarRocks Admin starting up");
    tracing::info!("Configuration loaded successfully");

    let pool = db::create_pool(&config.database.url).await?;
    tracing::info!("Database pool created successfully");

    // Start background tasks
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let service = services::headcount::HeadcountService::new(pool_clone);
        match service.sync_employees().await {
            Ok(count) => tracing::info!("Startup: Synced {} employees from OA", count),
            Err(e) => tracing::error!("Startup: Failed to sync employees from OA: {}", e),
        }
    });

    // Initialize core components
    let jwt_util = Arc::new(JwtUtil::new(&config.auth.jwt_secret, &config.auth.jwt_expires_in));
    let mysql_pool_manager = Arc::new(MySQLPoolManager::new());

    let auth_service = Arc::new(AuthService::new(pool.clone(), Arc::clone(&jwt_util)));

    let cluster_service =
        Arc::new(ClusterService::new(pool.clone(), Arc::clone(&mysql_pool_manager)));

    let organization_service = Arc::new(OrganizationService::new(pool.clone()));

    let system_function_service = Arc::new(SystemFunctionService::new(
        Arc::new(pool.clone()),
        Arc::clone(&mysql_pool_manager),
        Arc::clone(&cluster_service),
    ));

    // Create new services for cluster overview
    let metrics_collector_service = Arc::new(MetricsCollectorService::new(
        pool.clone(),
        Arc::clone(&cluster_service),
        Arc::clone(&mysql_pool_manager),
        config.metrics.retention_days,
    ));

    let data_statistics_service = Arc::new(DataStatisticsService::new(
        pool.clone(),
        Arc::clone(&cluster_service),
        Arc::clone(&mysql_pool_manager),
        config.audit.clone(),
    ));

    let overview_service = Arc::new(
        OverviewService::new(
            pool.clone(),
            Arc::clone(&cluster_service),
            Arc::clone(&mysql_pool_manager),
        )
        .with_data_statistics(Arc::clone(&data_statistics_service)),
    );

    // Initialize RBAC services
    let casbin_service = Arc::new(
        CasbinService::new()
            .await
            .map_err(|e| format!("Failed to initialize Casbin service: {}", e))?,
    );

    // Load initial policies from database
    casbin_service
        .reload_policies_from_db(&pool)
        .await
        .map_err(|e| format!("Failed to load initial policies: {}", e))?;
    tracing::info!("Casbin policies loaded from database");

    let permission_service =
        Arc::new(PermissionService::new(pool.clone(), Arc::clone(&casbin_service)));

    let role_service = Arc::new(RoleService::new(
        pool.clone(),
        Arc::clone(&casbin_service),
        Arc::clone(&permission_service),
    ));

    let user_role_service =
        Arc::new(UserRoleService::new(pool.clone(), Arc::clone(&casbin_service)));

    let user_service = Arc::new(UserService::new(pool.clone(), Arc::clone(&casbin_service)));

    let alert_service = Arc::new(AlertService::new(
        pool.clone(),
        Arc::clone(&mysql_pool_manager),
        config.audit.clone(),
    ));

    let application_service = Arc::new(ApplicationService::new(pool.clone()));
    let ai_service = Arc::new(AiService::new(pool.clone()));
    let asset_service = Arc::new(AssetService::new(pool.clone()));
    let duty_service = Arc::new(DutyService::new(pool.clone(), Arc::clone(&alert_service)));
    let system_service = Arc::new(SystemService::new(pool.clone()));

    // Build AppState with all services
    let app_state = AppState {
        db: pool.clone(),
        mysql_pool_manager: Arc::clone(&mysql_pool_manager),
        jwt_util: Arc::clone(&jwt_util),
        audit_config: config.audit.clone(),
        auth_service: Arc::clone(&auth_service),
        cluster_service: Arc::clone(&cluster_service),
        organization_service: Arc::clone(&organization_service),
        system_function_service: Arc::clone(&system_function_service),
        metrics_collector_service: Arc::clone(&metrics_collector_service),
        data_statistics_service: Arc::clone(&data_statistics_service),
        overview_service: Arc::clone(&overview_service),
        casbin_service: Arc::clone(&casbin_service),
        permission_service: Arc::clone(&permission_service),
        role_service: Arc::clone(&role_service),
        user_role_service: Arc::clone(&user_role_service),
        user_service: Arc::clone(&user_service),
        alert_service: Arc::clone(&alert_service),
        application_service: Arc::clone(&application_service),
        ai_service: Arc::clone(&ai_service),
        asset_service: Arc::clone(&asset_service),
        duty_service: Arc::clone(&duty_service),
        system_service: Arc::clone(&system_service),
    };

    // Start metrics collector using ScheduledExecutor (configurable interval)
    if config.metrics.enabled {
        let interval = std::time::Duration::from_secs(config.metrics.interval_secs);
        tracing::info!(
            "Starting metrics collector with interval: {}s (retention_days={})",
            config.metrics.interval_secs,
            config.metrics.retention_days
        );
        let executor = ScheduledExecutor::new("metrics-collector", interval);
        let service = Arc::clone(&metrics_collector_service);
        tokio::spawn(async move {
            executor.start(service).await;
        });
    } else {
        tracing::warn!("Metrics collector disabled by configuration");
    }

    // Start Alert Monitor
    let alert_service_bg = Arc::clone(&alert_service);
    tokio::spawn(async move {
        alert_service_bg.start_monitor_loop().await;
    });

    // Start SQL Fix Loop
    let alert_service_fix = Arc::clone(&alert_service);
    tokio::spawn(async move {
        alert_service_fix.start_sql_fix_loop().await;
    });

    // Start External Alert Sync (1m interval) - Disabled as requested
    /*
    let alert_service_sync = Arc::clone(&alert_service);
    let system_service_sync = Arc::clone(&system_service);
    tokio::spawn(async move {
        alert_service_sync
            .start_external_sync_loop(system_service_sync)
            .await;
    });
    */

    let duty_service_bg = Arc::clone(&duty_service);
    tokio::spawn(async move {
        duty_service_bg.start_monitor_loop().await;
    });

    let asset_service_sync = Arc::clone(&asset_service);
    tokio::spawn(async move {
        asset_service_sync.start_sync_loop().await;
    });

    // Wrap AppState in Arc for shared ownership across routes
    let app_state_arc = Arc::new(app_state);

    // Auth state for middleware (includes permission checking)
    let auth_state = middleware::AuthState {
        jwt_util: Arc::clone(&jwt_util),
        casbin_service: Arc::clone(&casbin_service),
        db: pool.clone(),
    };

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/share/sql/:query_id", get(handlers::alert::get_shared_sql))
        .route("/share/sql/:query_id/kill", post(handlers::alert::kill_shared_query))
        .route("/share/sql/:query_id/whitelist", post(handlers::alert::whitelist_shared_query))
        .route("/share/resource/panels", get(handlers::resource::list_panels))
        .route("/share/resource/datasources", get(handlers::resource::list_datasources))
        .route("/share/resource/query", post(handlers::resource::query_prometheus))
        .with_state(Arc::clone(&app_state_arc));

    // Protected routes (require authentication)
    let protected_routes = Router::new()
        // Auth
        .route("/api/auth/me", get(handlers::auth::get_me))
        .route("/api/auth/me", put(handlers::auth::update_me))
        // System Config
        .route("/api/system/config/:key", get(handlers::system::get_config))
        .route("/api/system/config/:key", put(handlers::system::update_config))
        // Clusters
        .route("/api/clusters", post(handlers::cluster::create_cluster))
        .route("/api/clusters", get(handlers::cluster::list_clusters))
        .route("/api/clusters/active", get(handlers::cluster::get_active_cluster))
        .route("/api/clusters/health/test", post(handlers::cluster::test_cluster_connection))
        // Backends
        .route("/api/clusters/backends", get(handlers::backend::list_backends))
        .route("/api/clusters/backends/:host/:port", delete(handlers::backend::delete_backend))
        // Frontends
        .route("/api/clusters/frontends", get(handlers::frontend::list_frontends))
        // Queries
        .route("/api/clusters/catalogs", get(handlers::query::list_catalogs))
        .route("/api/clusters/databases", get(handlers::query::list_databases))
        .route("/api/clusters/tables", get(handlers::query::list_tables))
        .route(
            "/api/clusters/catalogs-databases",
            get(handlers::query::list_catalogs_with_databases),
        )
        .route("/api/clusters/queries", get(handlers::query::list_queries))
        .route("/api/clusters/queries/execute", post(handlers::query::execute_sql))
        .route("/api/clusters/queries/:query_id", delete(handlers::query::kill_query))
        .route("/api/clusters/queries/history", get(handlers::query_history::list_query_history))
        // Cluster detail routes (placed after specific query routes to avoid path conflicts)
        .route("/api/clusters/:id", get(handlers::cluster::get_cluster))
        .route("/api/clusters/:id", put(handlers::cluster::update_cluster))
        .route("/api/clusters/:id", delete(handlers::cluster::delete_cluster))
        .route("/api/clusters/:id/activate", put(handlers::cluster::activate_cluster))
        .route(
            "/api/clusters/:id/health",
            get(handlers::cluster::get_cluster_health).post(handlers::cluster::get_cluster_health),
        )
        // Organizations
        .route(
            "/api/organizations",
            post(handlers::organization::create_organization)
                .get(handlers::organization::list_organizations),
        )
        .route(
            "/api/organizations/:id",
            get(handlers::organization::get_organization)
                .put(handlers::organization::update_organization)
                .delete(handlers::organization::delete_organization),
        )
        // Materialized Views
        .route(
            "/api/clusters/materialized_views",
            get(handlers::materialized_view::list_materialized_views)
                .post(handlers::materialized_view::create_materialized_view),
        )
        .route(
            "/api/clusters/materialized_views/:mv_name",
            get(handlers::materialized_view::get_materialized_view)
                .delete(handlers::materialized_view::delete_materialized_view)
                .put(handlers::materialized_view::alter_materialized_view),
        )
        .route(
            "/api/clusters/materialized_views/:mv_name/ddl",
            get(handlers::materialized_view::get_materialized_view_ddl),
        )
        .route(
            "/api/clusters/materialized_views/:mv_name/refresh",
            post(handlers::materialized_view::refresh_materialized_view),
        )
        .route(
            "/api/clusters/materialized_views/:mv_name/cancel",
            post(handlers::materialized_view::cancel_refresh_materialized_view),
        )
        // Profiles
        .route("/api/clusters/profiles", get(handlers::profile::list_profiles))
        .route("/api/clusters/profiles/:query_id", get(handlers::profile::get_profile))
        .route(
            "/api/clusters/profiles/:query_id/analyze",
            get(handlers::profile::analyze_profile_handler),
        )
        // Sessions
        .route("/api/clusters/sessions", get(handlers::sessions::get_sessions))
        .route("/api/clusters/sessions/:session_id", delete(handlers::sessions::kill_session))
        // Variables
        .route("/api/clusters/variables", get(handlers::variables::get_variables))
        .route("/api/clusters/variables/:variable_name", put(handlers::variables::update_variable))
        // System
        .route("/api/clusters/system/runtime_info", get(handlers::system::get_runtime_info))
        .route("/api/clusters/system", get(handlers::system_management::get_system_functions))
        .route(
            "/api/clusters/system/:function_name",
            get(handlers::system_management::get_system_function_detail),
        )
        // System Functions
        .route(
            "/api/clusters/system-functions",
            get(handlers::system_function::get_system_functions)
                .post(handlers::system_function::create_system_function),
        )
        .route(
            "/api/clusters/system-functions/orders",
            put(handlers::system_function::update_function_orders),
        )
        .route(
            "/api/clusters/system-functions/:function_id/execute",
            post(handlers::system_function::execute_system_function),
        )
        .route(
            "/api/clusters/system-functions/:function_id/favorite",
            put(handlers::system_function::toggle_function_favorite),
        )
        .route(
            "/api/clusters/system-functions/:function_id",
            put(handlers::system_function::update_function)
                .delete(handlers::system_function::delete_system_function),
        )
        .route(
            "/api/system-functions/:function_name/access-time",
            put(handlers::system_function::update_system_function_access_time),
        )
        .route(
            "/api/system-functions/category/:category_name",
            delete(handlers::system_function::delete_category),
        )
        // Overview
        .route("/api/clusters/overview", get(handlers::overview::get_cluster_overview))
        .route(
            "/api/clusters/overview/extended",
            get(handlers::overview::get_extended_cluster_overview),
        )
        .route("/api/clusters/overview/health", get(handlers::overview::get_health_cards))
        .route(
            "/api/clusters/overview/performance",
            get(handlers::overview::get_performance_trends),
        )
        .route("/api/clusters/overview/resources", get(handlers::overview::get_resource_trends))
        .route("/api/clusters/overview/data-stats", get(handlers::overview::get_data_statistics))
        .route(
            "/api/clusters/overview/capacity-prediction",
            get(handlers::overview::get_capacity_prediction),
        )
        .route(
            "/api/clusters/overview/compaction-details",
            get(handlers::overview::get_compaction_detail_stats),
        )
        // RBAC Routes
        // Roles
        .route("/api/roles", get(handlers::role::list_roles).post(handlers::role::create_role))
        .route(
            "/api/roles/:id",
            get(handlers::role::get_role)
                .put(handlers::role::update_role)
                .delete(handlers::role::delete_role),
        )
        .route(
            "/api/roles/:id/permissions",
            get(handlers::role::get_role_with_permissions)
                .put(handlers::role::update_role_permissions),
        )
        // Permissions
        .route("/api/permissions", get(handlers::permission::list_permissions))
        .route("/api/permissions/menu", get(handlers::permission::list_menu_permissions))
        .route("/api/permissions/api", get(handlers::permission::list_api_permissions))
        .route("/api/permissions/tree", get(handlers::permission::get_permission_tree))
        .route("/api/auth/permissions", get(handlers::permission::get_current_user_permissions))
        // User Management
        .route("/api/users", get(handlers::user::list_users).post(handlers::user::create_user))
        .route(
            "/api/users/:id",
            get(handlers::user::get_user)
                .put(handlers::user::update_user)
                .delete(handlers::user::delete_user),
        )
        // User Roles
        .route(
            "/api/users/:id/roles",
            get(handlers::user_role::get_user_roles).post(handlers::user_role::assign_role_to_user),
        )
        .route("/api/users/:id/roles/:role_id", delete(handlers::user_role::remove_role_from_user))
        // Duty Routes
        .route(
            "/api/duty/personnel",
            get(handlers::duty::list_personnel).post(handlers::duty::create_personnel),
        )
        .route(
            "/api/duty/personnel/:id",
            put(handlers::duty::update_personnel).delete(handlers::duty::delete_personnel),
        )
        .route("/api/duty/schedule", get(handlers::duty::get_schedule))
        .route("/api/duty/schedule/:id", delete(handlers::duty::delete_schedule))
        .route("/api/duty/schedule/batch", post(handlers::duty::batch_assign_schedule))
        .route(
            "/api/duty/rotation",
            get(handlers::duty::list_rotations).post(handlers::duty::save_rotation),
        )
        .route("/api/duty/rotation/config", put(handlers::duty::update_rotation_config))
        .route("/api/duty/notify-manual", post(handlers::duty::notify_manual))
        // Headcount
        .route("/api/headcount/employees", get(handlers::headcount::list_employees))
        .route("/api/headcount/sync", post(handlers::headcount::sync_employees))
        // Asset Inventory
        .route(
            "/api/asset/resources",
            get(handlers::asset::list_resources).put(handlers::asset::update_resource),
        )
        .route("/api/asset/resources/batch-delete", post(handlers::asset::delete_resources))
        .route("/api/asset/import", post(handlers::asset::import_resources))
        .route("/api/asset/filter-options", get(handlers::asset::get_filter_options))
        .route("/api/asset/apply", post(handlers::asset::apply_resources))
        .route("/api/asset/service-op", post(handlers::asset::service_operation))
        // Resource Management Routes
        .route(
            "/api/resource/panels",
            get(handlers::resource::list_panels).post(handlers::resource::create_panel),
        )
        .route(
            "/api/resource/panels/:id",
            put(handlers::resource::update_panel).delete(handlers::resource::delete_panel),
        )
        .route(
            "/api/resource/datasources",
            get(handlers::resource::list_datasources).post(handlers::resource::create_datasource),
        )
        .route("/api/resource/datasources/test", post(handlers::resource::test_datasource))
        .route(
            "/api/resource/datasources/:id",
            put(handlers::resource::update_datasource)
                .delete(handlers::resource::delete_datasource),
        )
        .route("/api/resource/query", post(handlers::resource::query_prometheus))
        .route(
            "/api/resource/settings",
            get(handlers::resource::get_settings).put(handlers::resource::update_settings),
        )
        // Alert Management Routes
        .route(
            "/api/alert/rules",
            get(handlers::alert::list_rules).post(handlers::alert::create_rule),
        )
        .route(
            "/api/alert/rules/:id",
            put(handlers::alert::update_rule).delete(handlers::alert::delete_rule),
        )
        .route("/api/alert/rules/:id/test", post(handlers::alert::test_alert))
        .route("/api/alert/notify", post(handlers::alert::notify))
        .route("/api/alert/history", get(handlers::alert::list_history))
        .route("/api/alert/history/:id", get(handlers::alert::get_history))
        .route("/api/alert/history/clusters", get(handlers::alert::list_history_clusters))
        .route("/api/alert/history/departments", get(handlers::alert::list_history_departments))
        .route("/api/alert/summary/sql", get(handlers::alert::get_sql_summary))
        .route("/api/alert/summary/sql/trend", get(handlers::alert::get_sql_trend))
        .route(
            "/api/alert/summary/external",
            get(handlers::alert::get_external_summary).post(handlers::alert::proxy_webhook),
        )
        .route("/api/alert/history/:id/kill", post(handlers::alert::kill_query))
        .route("/api/alert/history/:id/whitelist", post(handlers::alert::whitelist_query))
        .route("/api/alert/history/:id/remark", put(handlers::alert::update_remark))
        .route("/api/alert/history/:id/repair_person", put(handlers::alert::update_repair_person))
        // Application Management Routes
        .route(
            "/api/applications",
            get(handlers::application::list_applications)
                .post(handlers::application::create_application),
        )
        .route(
            "/api/applications/:id",
            put(handlers::application::update_application)
                .delete(handlers::application::delete_application),
        )
        // AI Settings Routes
        .route(
            "/api/ai/settings",
            get(handlers::ai::list_ai_settings).post(handlers::ai::create_ai_setting),
        )
        .route(
            "/api/ai/settings/:id",
            put(handlers::ai::update_ai_setting).delete(handlers::ai::delete_ai_setting),
        )
        // Data Sync Routes
        .route("/api/data-sync/submit", post(handlers::data_sync::submit_sync_ticket))
        .route("/api/data-sync/proxy-webhook", post(handlers::data_sync::proxy_webhook))
        .route("/api/data-sync/list", get(handlers::data_sync::list_tickets))
        .route("/api/data-sync/list/:id/processor", put(handlers::data_sync::update_processor))
        .route("/api/data-sync/list/:id/status", put(handlers::data_sync::update_status))
        .route("/api/data-sync/list/:id/approve", put(handlers::data_sync::approve_sync_ticket))
        .with_state(Arc::clone(&app_state_arc))
        .layer(axum_middleware::from_fn_with_state(auth_state, middleware::auth_middleware));

    let health_routes = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(ready_check));

    // Static file serving from embedded assets
    let static_routes = if config.static_config.enabled {
        tracing::info!("Static file serving enabled, serving from embedded assets");
        Router::new().fallback(serve_static_files)
    } else {
        Router::new()
    };

    // Build the main app router
    let app = Router::new()
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(public_routes)
        .merge(protected_routes)
        .merge(health_routes)
        .merge(static_routes); // Must be last to serve as fallback for SPA routes

    let app = app
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive());

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API documentation available at http://{}/api-docs", addr);
    tracing::info!("StarRocks Admin is ready to serve requests");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn ready_check() -> &'static str {
    "READY"
}

/// Serve static files from embedded assets
/// Handles SPA routing by falling back to index.html for non-API routes
///
/// Flink-style implementation: backend is path-agnostic,
/// relies on reverse proxy (Nginx/Traefik) rewrite rules
///
/// For sub-path deployments, static assets may be requested with route segments in the path
/// (e.g., /starrocks-admin/pages/starrocks/runtime.js). This function extracts the filename
/// from such paths to correctly serve static assets.
async fn serve_static_files(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Don't serve static files for API routes
    if path.starts_with("api/") || path.starts_with("api-docs/") {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }

    // Check if this is a static asset request (has file extension)
    // If the path contains route segments but ends with a static file extension,
    // extract just the filename to serve the correct asset
    let static_extensions = [
        "js", "css", "png", "jpg", "jpeg", "gif", "svg", "ico", "woff", "woff2", "ttf", "eot",
        "otf", "json",
    ];
    let is_static_asset = static_extensions
        .iter()
        .any(|ext| path.ends_with(&format!(".{}", ext)));

    // Determine which asset path to use
    let mut requested_asset = None;

    if let Some(file) = WebAssets::get(path) {
        // Found at the full path
        requested_asset = Some((file, path));
    } else if is_static_asset {
        // Fallback: Extract filename from path (handles cases like /starrocks-admin/pages/starrocks/runtime.js)
        let fallback_path = path
            .split('/')
            .next_back()
            .filter(|s| s.contains('.'))
            .map(|s| s.to_string());

        if let Some(file) = fallback_path
            .filter(|p| p != path)
            .and_then(|f_path| WebAssets::get(&f_path))
        {
            requested_asset = Some((file, path)); // Keep original path for content-type
        }
    }

    // Serve the asset if found
    if let Some((file, type_path)) = requested_asset {
        let content_type = get_content_type(type_path);
        let data: Vec<u8> = file.data.to_vec();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .unwrap()
            .into_response();
    }

    // For SPA routing, fall back to index.html for any non-API route
    // Frontend uses relative API paths (./api), so it works with any deployment path
    if let Some(index) = WebAssets::get("index.html") {
        let data: Vec<u8> = index.data.to_vec();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(data))
            .unwrap()
            .into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

/// Get content type based on file extension
fn get_content_type(path: &str) -> HeaderValue {
    let ext = path.rsplit('.').next().unwrap_or("");
    let content_type = match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    };
    HeaderValue::from_static(content_type)
}
