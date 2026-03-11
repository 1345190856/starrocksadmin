use crate::models::Cluster;
use crate::models::alert::{
    AlertChannel, AlertHistory, AlertHistoryResponse, AlertReceiver, AlertRule,
    CreateAlertRuleRequest, HistoryQuery, UpdateAlertRuleRequest,
};
use crate::models::resource::ResourceDataSource;
use crate::services::MySQLPoolManager;
use crate::utils::{ApiError, ApiResult};
use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};
use dashmap::DashMap;
use serde_json::json;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{error, info};

#[derive(Clone)]
pub struct AlertService {
    pool: MySqlPool,
    mysql_pool_manager: Arc<MySQLPoolManager>,
    audit_config: crate::config::AuditLogConfig,
    // State: QueryId -> ActiveQueryState
    // Key: "ClusterID_QueryID" to avoid collision across clusters
    active_queries: Arc<DashMap<String, ActiveQueryState>>,
}

#[derive(Debug, Clone)]
struct ActiveQueryState {
    query_id: String,
    #[allow(dead_code)]
    datasource_id: i32,
    datasource_name: String,
    start_time: String,
    last_seen: DateTime<Utc>,

    // Latest stats
    connection_id: String,
    user: String,
    db: String,
    sql_text: String,
    fe_ip: String,

    scan_rows: i64,
    scan_bytes: i64,
    memory_usage: i64, // bytes
    cpu_time: f64,     // seconds
    exec_time: f64,    // seconds

    // Alert state per rule/channel
    channel_alert_times: std::collections::HashMap<(i32, String), DateTime<Utc>>,
    channel_alert_counts: std::collections::HashMap<(i32, String), i32>,
    first_alert_time: Option<DateTime<Utc>>, // First time the query violated ANY rule
    auto_kill_notified: bool,                // Prevent duplicate kill notifications
    resolved_notified: bool,                 // Prevent duplicate resolution notifications
}

impl AlertService {
    pub fn new(
        pool: MySqlPool,
        mysql_pool_manager: Arc<MySQLPoolManager>,
        audit_config: crate::config::AuditLogConfig,
    ) -> Self {
        Self { pool, mysql_pool_manager, audit_config, active_queries: Arc::new(DashMap::new()) }
    }

    // --- Background Task ---

    pub async fn start_monitor_loop(self: Arc<Self>) {
        info!("Starting Alert Monitor Loop");
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_clusters().await {
                error!("Error in alert monitor loop: {}", e);
            }
        }
    }

    pub async fn start_sql_fix_loop(self: Arc<Self>) {
        info!("Starting Alert SQL Text Fix Loop (5m interval)");
        let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;
            if let Err(e) = self.fix_missing_sql_texts().await {
                error!("Error in alert sql fix loop: {}", e);
            }
        }
    }

    async fn fix_missing_sql_texts(&self) -> ApiResult<()> {
        #[derive(sqlx::FromRow)]
        struct MissingSqlRow {
            id: i32,
            query_id: String,
            ds_id: i32,
            ds_name: String,
            ds_url: String,
            ds_user: Option<String>,
            ds_pass: Option<String>,
            ds_fe_mapping: Option<String>,
            ds_timeout: Option<i32>,
        }

        // 1. Find history records with missing SQL (Full scan)
        let rows: Vec<MissingSqlRow> = sqlx::query_as(
            r#"
            SELECT 
                h.id, h.query_id, 
                ds.id as ds_id, ds.name as ds_name, ds.url as ds_url, 
                ds.username as ds_user, ds.password as ds_pass, 
                CAST(ds.fe_mapping AS CHAR) as ds_fe_mapping, ds.connection_timeout as ds_timeout
            FROM warning_rule.alert_history h
            JOIN warning_rule.alert_rules r ON h.rule_id = r.id
            JOIN resource_data_sources ds ON r.datasource_id = ds.id
            WHERE (h.sql_text IS NULL OR h.sql_text = '' OR h.sql_text = 'Unknown')
            ORDER BY h.id DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        if rows.is_empty() {
            return Ok(());
        }

        info!(
            "[SQL Fix Task] Found {} records with missing SQL text, attempting recovery from audit logs...",
            rows.len()
        );

        for row in rows {
            // Parse fe_mapping manually to avoid sqlx decoding issues with NULLs
            let fe_mapping: Option<serde_json::Value> = row
                .ds_fe_mapping
                .and_then(|s| serde_json::from_str(&s).ok());

            // Reconstruct minimal ResourceDataSource for ds_to_cluster
            let ds = ResourceDataSource {
                id: row.ds_id,
                name: row.ds_name,
                r#type: "starrocks".to_string(),
                url: row.ds_url,
                username: row.ds_user,
                password: row.ds_pass,
                region: None,
                fe_mapping,
                connection_timeout: row.ds_timeout,
            };

            let cluster = self.ds_to_cluster(&ds);

            // Background task always uses audit log directly using default connection
            if let Some(sql) = self.fetch_from_audit_log(&cluster, &row.query_id).await {
                info!(
                    "[SQL Fix Task] Successfully recovered SQL for history ID {} (query_id: {})",
                    row.id, row.query_id
                );
                sqlx::query("UPDATE warning_rule.alert_history SET sql_text = ? WHERE id = ?")
                    .bind(sql)
                    .bind(row.id)
                    .execute(&self.pool)
                    .await
                    .map_err(ApiError::database_error)?;
            }

            // Control frequency to avoid overwhelming the audit log
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    pub async fn check_all_clusters(&self) -> ApiResult<()> {
        // 1. Get enabled rules to know which clusters/regions to check
        let rules = self.get_enabled_rules().await?;
        if rules.is_empty() {
            tracing::info!("[Altrt Monitor Task] Heartbeat: No enabled rules found.");
            return Ok(());
        }

        let datasources: Vec<ResourceDataSource> =
            sqlx::query_as("SELECT * FROM resource_data_sources WHERE type = 'starrocks'")
                .fetch_all(&self.pool)
                .await
                .map_err(ApiError::database_error)?;

        tracing::info!(
            "[Alert Monitor Task] Heartbeat: {} starrocks data sources, {} enabled rules.",
            datasources.len(),
            rules.len()
        );

        let rules = Arc::new(rules);
        let mut set = tokio::task::JoinSet::new();

        for ds in datasources {
            // Filter rules that match this data source by ID or by Name (fallback)
            let ds_rules: Vec<AlertRule> = rules
                .iter()
                .filter(|r| {
                    if let Some(rid) = r.datasource_id {
                        rid == ds.id
                    } else {
                        r.data_source == ds.name
                    }
                })
                .cloned()
                .collect();

            if ds_rules.is_empty() {
                continue;
            }

            let service = self.clone();
            set.spawn(async move {
                let rule_refs: Vec<&AlertRule> = ds_rules.iter().collect();
                tracing::info!(
                    "[Alert Monitor] Checking data source {} (ID: {}) with {} rules",
                    ds.name,
                    ds.id,
                    rule_refs.len()
                );

                let cluster = service.ds_to_cluster(&ds);
                // Check this data source with timeout
                let result = time::timeout(
                    Duration::from_secs(30),
                    service.check_cluster(&cluster, &rule_refs),
                )
                .await;

                match result {
                    Ok(Ok(_)) => Some(ds.id),
                    Ok(Err(e)) => {
                        tracing::error!(
                            "[Alert Monitor] Connection/Query Error on {}: {}. Queries preserved.",
                            ds.name,
                            e
                        );
                        None
                    },
                    Err(_) => {
                        tracing::error!(
                            "[Alert Monitor] Timeout checking {} (30s). Queries preserved.",
                            ds.name
                        );
                        None
                    },
                }
            });
        }

        let mut successful_ds_ids = std::collections::HashSet::new();
        while let Some(res) = set.join_next().await {
            if let Ok(Some(ds_id)) = res {
                successful_ds_ids.insert(ds_id);
            }
        }

        // 3. Cleanup stale queries - only for data sources we actually successfully talked to
        self.cleanup_stale_queries(successful_ds_ids).await;

        // 4. Cleanup orphaned alerts
        if let Err(e) = self.cleanup_orphaned_alerts().await {
            tracing::error!("Failed to cleanup orphaned alerts: {}", e);
        }

        Ok(())
    }

    async fn cleanup_orphaned_alerts(&self) -> ApiResult<()> {
        // 1. Get all "Alerting" records from DB
        let alerting_records: Vec<AlertHistory> = sqlx::query_as(
            "SELECT * FROM warning_rule.alert_history WHERE status IN ('Alerting', 'Suppressed')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        if alerting_records.is_empty() {
            return Ok(());
        }

        // 2. Check against active_queries
        // active_queries contains keys "ClusterID_QueryID".
        // history record has host (cluster name) and query_id.
        // We need to resolve host -> cluster_id to form the key, or just check if query_id exists in any value.
        // Checking values is slower (O(N)), but N is small (active queries).

        // Optimization: Build a set of active Query IDs to check against.
        // Note: query_id might not be unique across clusters, but practically usually is or we check cluster.

        let active_set: std::collections::HashSet<String> = self
            .active_queries
            .iter()
            .map(|entry| entry.value().query_id.clone())
            .collect();

        for record in alerting_records {
            // If record.query_id is NOT in active_set, it means the query is finished/gone.
            // (Assuming active_queries is fully populated by check_all_clusters right before this)

            if !active_set.contains(&record.query_id) {
                tracing::info!(
                    "[Alert Monitor] Found orphaned alert record (QueryId: {}), marking as Resolved",
                    record.query_id
                );

                sqlx::query("UPDATE warning_rule.alert_history SET status = 'Resolved', sql_text = COALESCE(sql_text, 'Query finished while monitor was down') WHERE id = ?")
                    .bind(record.id)
                    .execute(&self.pool)
                    .await
                    .map_err(ApiError::database_error)?;
            }
        }

        Ok(())
    }

    async fn check_cluster(&self, cluster: &Cluster, rules: &[&AlertRule]) -> ApiResult<()> {
        use mysql_async::prelude::Queryable;
        let version = &rules[0].starrocks_version;

        let is_global = version != "3.2";

        let mut rows = Vec::new();

        if is_global {
            // Case 1: show proc '/global_current_queries' -> Fixed to default FE
            let pool = self.mysql_pool_manager.get_pool(cluster).await?;
            let mut conn = pool
                .get_conn()
                .await
                .map_err(|e| ApiError::internal_error(format!("Failed to connect to SR: {}", e)))?;

            rows = conn
                .query("show proc '/global_current_queries'")
                .await
                .map_err(|e| {
                    ApiError::internal_error(format!(
                        "Failed to show proc /global_current_queries: {}",
                        e
                    ))
                })?;
        } else {
            // Case 2: show proc '/current_queries' -> Poll all mapped FEs directly
            let mut target_endpoints = Vec::new();

            if let Some(mapping_obj) = cluster.fe_mapping.as_ref().and_then(|m| m.as_object()) {
                for val in mapping_obj.values().filter_map(|v| v.as_str()) {
                    let parts: Vec<&str> = val.split(':').collect();
                    let host = parts[0].to_string();
                    let port = parts
                        .get(1)
                        .and_then(|p| p.parse::<i32>().ok())
                        .unwrap_or(cluster.fe_query_port);
                    target_endpoints.push((host, port));
                }
            }

            // Fallback: If no mappings defined, use the default FE
            if target_endpoints.is_empty() {
                target_endpoints.push((cluster.fe_host.clone(), cluster.fe_query_port));
            } else {
                target_endpoints.sort();
                target_endpoints.dedup();
            }

            for (target_host, target_port) in target_endpoints {
                // Connect to each specific FE
                let mut fe_cluster = cluster.clone();

                // Unique pool ID for each FE endpoint (stable)
                let fe_pool_id = {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    cluster.id.hash(&mut hasher);
                    target_host.hash(&mut hasher);
                    target_port.hash(&mut hasher);
                    let h = hasher.finish();
                    -((h & 0x7FFFFFFFFFFFFFFF) as i64).max(1)
                };
                fe_cluster.id = fe_pool_id;
                fe_cluster.fe_host = target_host;
                fe_cluster.fe_query_port = target_port;

                if let Ok(fe_pool) = self.mysql_pool_manager.get_pool(&fe_cluster).await {
                    // Short timeout for individual FE scan
                    #[allow(clippy::collapsible_if)]
                    if let Ok(Ok(mut fe_conn)) =
                        time::timeout(Duration::from_secs(5), fe_pool.get_conn()).await
                    {
                        if let Ok(fe_data) = fe_conn
                            .query::<mysql_async::Row, _>("show proc '/current_queries'")
                            .await
                        {
                            rows.extend(fe_data);
                        }
                    }
                }
            }
        }

        if self.audit_config.debug {
            tracing::info!(
                "[Alert Monitor Debug] Cluster: {} (ID: {}), Mode: {}, Results: {} rows",
                cluster.name,
                cluster.id,
                if is_global { "Global" } else { "Polled" },
                rows.len()
            );
        }

        if rows.is_empty() {
            return Ok(());
        }

        let now = Utc::now();

        let row_columns = rows[0].columns_ref().to_vec();
        // Helper to safely get optional string columns
        let get_row_val = |row: &mysql_async::Row, names: &[&str]| -> String {
            for name in names {
                // Find case-insensitive match for performance and robustness
                #[allow(clippy::collapsible_if)]
                if let Some(pos) = row_columns
                    .iter()
                    .position(|c| c.name_str().eq_ignore_ascii_case(name))
                {
                    if let Some(val) = row.get::<Option<String>, usize>(pos).flatten() {
                        if !val.is_empty() {
                            return val;
                        }
                    }
                }
            }
            String::new()
        };

        for row in rows {
            let query_id = get_row_val(&row, &["QueryId", "ID"]);
            if query_id.is_empty() {
                continue;
            }

            let fe_ip = get_row_val(&row, &["feIp", "Host"]);
            let conn_id = get_row_val(&row, &["ConnectionId", "conn_id"]);

            let key = format!("{}_{}", cluster.id, query_id);

            // If ConnectionId is "0", the query is actually finished
            if conn_id == "0" {
                if let Some((_, mut state)) = self.active_queries.remove(&key) {
                    // Try to fetch full SQL one last time if missing
                    #[allow(clippy::collapsible_if)]
                    if state.sql_text.is_empty() || state.sql_text.contains("...") {
                        if let Some(full_sql) = self
                            .fetch_full_sql(cluster, &state.query_id, Some(&state.fe_ip))
                            .await
                        {
                            state.sql_text = full_sql;
                        }
                    }
                    let _ = self.resolve_active_query(&state).await;
                }
                continue;
            }
            let user = get_row_val(&row, &["User", "user"]);
            let db = get_row_val(&row, &["Database", "db"]);
            let mem_str = get_row_val(&row, &["MemoryUsage", "MemUsage"]);
            let cpu_str = get_row_val(&row, &["CPUTime", "CpuTime"]);
            let exec_str = get_row_val(&row, &["ExecTime", "ExecutionTime"]);
            let scan_rows_str = get_row_val(&row, &["ScanRows", "scan_rows"]);
            let scan_bytes_str = get_row_val(&row, &["ScanBytes", "scan_bytes"]);

            let sql_text = get_row_val(&row, &["Sql", "Stmt", "Info"]);

            // Parse values
            let mem_bytes = parse_size(&mem_str);
            let cpu_sec = parse_time(&cpu_str);
            let exec_sec = parse_time(&exec_str);
            let scan_rows = parse_count(&scan_rows_str);
            let scan_bytes = parse_size(&scan_bytes_str);

            let start_time_str = get_row_val(&row, &["StartTime", "start_time"]);
            let start_time = if !start_time_str.is_empty() {
                start_time_str
            } else {
                now.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            };

            // Debug logging for parsing
            if !rules.is_empty() {
                tracing::debug!(
                    "Parsed query {}: CPU='{}'->{}s, Mem='{}'->{}, ScanRows='{}'->{}, Exec='{}'->{}s",
                    query_id,
                    cpu_str,
                    cpu_sec,
                    mem_str,
                    mem_bytes,
                    scan_rows_str,
                    scan_rows,
                    exec_str,
                    exec_sec
                );
            }

            // 1. Update/Get state and release lock immediately
            let mut state = {
                let mut entry =
                    self.active_queries
                        .entry(key.clone())
                        .or_insert_with(|| ActiveQueryState {
                            query_id: query_id.clone(),
                            datasource_id: cluster.id as i32,
                            datasource_name: cluster.name.clone(),
                            start_time,
                            last_seen: now,
                            connection_id: conn_id.clone(),
                            user: user.clone(),
                            db: db.clone(),
                            sql_text: sql_text.clone(),
                            fe_ip: fe_ip.clone(),
                            scan_rows,
                            scan_bytes,
                            memory_usage: mem_bytes,
                            cpu_time: cpu_sec,
                            exec_time: exec_sec,
                            channel_alert_times: std::collections::HashMap::new(),
                            channel_alert_counts: std::collections::HashMap::new(),
                            first_alert_time: None,
                            auto_kill_notified: false,
                            resolved_notified: false,
                        });

                let s = entry.value_mut();
                s.last_seen = now;
                s.scan_rows = scan_rows;
                s.memory_usage = mem_bytes;
                s.cpu_time = cpu_sec;
                s.exec_time = exec_sec;
                s.scan_bytes = scan_bytes;
                if !sql_text.is_empty() {
                    s.sql_text = sql_text.clone();
                }
                if !fe_ip.is_empty() {
                    s.fe_ip = fe_ip.clone();
                }
                s.connection_id = conn_id.clone();
                s.user = user.clone();
                s.db = db.clone();

                s.clone()
            }; // RefMut dropped here, lock released

            // 2. Perform async rule evaluation (no map lock held)
            self.evaluate_rules(cluster, &mut state, rules).await;

            // 3. Update alert-specific state back to map
            if let Some(mut entry) = self.active_queries.get_mut(&key) {
                let s = entry.value_mut();
                s.channel_alert_times = state.channel_alert_times;
                s.channel_alert_counts = state.channel_alert_counts;
                s.first_alert_time = state.first_alert_time;
                s.auto_kill_notified = state.auto_kill_notified;
                s.resolved_notified = state.resolved_notified;
                s.sql_text = state.sql_text;
                s.last_seen = Utc::now(); // Update last_seen with fresh time after async work
            }
        }

        Ok(())
    }

    async fn cleanup_stale_queries(&self, successful_ds_ids: std::collections::HashSet<i32>) {
        // Remove queries not seen in last 5 minutes (allowing for slow/stalled scans)
        let threshold = Utc::now() - ChronoDuration::minutes(5);

        // 1. Identify disappeared queries and decide their fate
        let mut to_resolve = Vec::new();
        let mut to_remove_from_memory = Vec::new();

        for entry in self.active_queries.iter() {
            let state = entry.value();
            if state.last_seen <= threshold {
                let total_alerts: i32 = state.channel_alert_counts.values().sum();

                if total_alerts > 0 {
                    // This was an alerting or killed query.
                    // We only resolve and remove it if we confirmed via a successful scan that it's actually gone.
                    if successful_ds_ids.contains(&state.datasource_id) {
                        // CROSS-CHECK: Only resolve in DB if NO OTHER datasource still sees this query as active/fresh
                        let seen_elsewhere = self.active_queries.iter().any(|other| {
                            let (o_key, o_state) = (other.key(), other.value());
                            o_key != entry.key()
                                && o_state.query_id == state.query_id
                                && o_state.last_seen > threshold
                        });

                        if !seen_elsewhere {
                            to_resolve.push(state.clone());
                        } else {
                            tracing::info!(
                                "[Alert Monitor] Query {} stale on DS {}, but still seen on another FE. Skipping DB resolution.",
                                state.query_id,
                                state.datasource_id
                            );
                        }
                        to_remove_from_memory.push(entry.key().clone());
                    } else if self.audit_config.debug {
                        tracing::debug!(
                            "[Alert Monitor] Query {} is stale but its data source (ID: {}) was not reachable. Preserving state.",
                            state.query_id,
                            state.datasource_id
                        );
                    }
                } else {
                    // Just normal query tracking data that hasn't been seen.
                    // Safe to remove from memory to prevent leaks.
                    to_remove_from_memory.push(entry.key().clone());
                }
            }
        }

        // 2. Mark as Resolved in DB
        for state in to_resolve {
            let _ = self.resolve_active_query(&state).await;
        }

        // 3. Finally remove from memory
        for key in to_remove_from_memory {
            self.active_queries.remove(&key);
        }
    }

    async fn resolve_active_query(&self, state: &ActiveQueryState) -> ApiResult<()> {
        let mut final_sql = state.sql_text.clone();

        // If SQL is missing, try one last fetch from audit log now that query is finished
        #[allow(clippy::collapsible_if)]
        if final_sql.is_empty() || final_sql.contains("...") {
            // Fetch the data source to get connection info
            let ds_res: Option<ResourceDataSource> =
                sqlx::query_as("SELECT * FROM resource_data_sources WHERE id = ?")
                    .bind(state.datasource_id)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap_or(None);

            if let Some(ds) = ds_res {
                let cluster = self.ds_to_cluster(&ds);
                if let Some(full_sql) = self
                    .fetch_full_sql(&cluster, &state.query_id, Some(&state.fe_ip))
                    .await
                {
                    final_sql = full_sql;
                }
            }
        }

        // Update SQL and Status in history
        let _ = sqlx::query("UPDATE warning_rule.alert_history SET sql_text = ?, status = 'Resolved' WHERE query_id = ? AND status IN ('Alerting', 'KillFailed', 'Suppressed')")
            .bind(&final_sql)
            .bind(&state.query_id)
            .execute(&self.pool)
            .await;

        Ok(())
    }

    async fn evaluate_rules(
        &self,
        cluster: &Cluster,
        state: &mut ActiveQueryState,
        rules: &[&AlertRule],
    ) {
        let mut any_violation = false; // Track if *any* rule is violated for this query
        for rule in rules {
            if self.audit_config.debug {
                tracing::info!(
                    "[Alert Monitor Debug] Evaluating rule '{}' (ID: {}) for query '{}'",
                    rule.name,
                    rule.id,
                    state.query_id
                );
            }
            if !self.is_rule_applicable(rule, state) {
                continue;
            }

            if self.check_violation(rule, state) {
                any_violation = true;
                self.process_alert(cluster, rule, state).await;
            }
        }

        // Auto-Resolution: If it was alerting but now it's not violating any rule
        // (Decision: We only resolve the entire query's alert status if NO rules are violated anymore)
        let total_alerts: i32 = state.channel_alert_counts.values().sum();
        if !any_violation && total_alerts > 0 {
            // CROSS-CHECK: Even if this FE says it's not violating, don't resolve in DB if ANOTHER FE still sees a violation.
            // This prevents status flickering due to FE sync lag or transient metric drops.
            let violating_elsewhere = self.active_queries.iter().any(|other| {
                let o_state = other.value();
                o_state.query_id == state.query_id
                    && o_state.datasource_id != state.datasource_id
                    && o_state.channel_alert_counts.values().sum::<i32>() > 0
            });

            if !violating_elsewhere {
                tracing::info!(
                    "[Alert Monitor] Query {} ({}s) no longer violates any rule. Resolving (previous alerts: {}).",
                    state.query_id,
                    state.exec_time,
                    total_alerts
                );
                let _ = self.resolve_active_query(state).await;
            } else {
                tracing::info!(
                    "[Alert Monitor] Query {} no longer violates rule on DS {}, but another FE still reports violation. Skipping resolution.",
                    state.query_id,
                    state.datasource_id
                );
            }

            state.channel_alert_counts.clear();
            state.channel_alert_times.clear();
            state.first_alert_time = None;
        }
    }

    fn is_rule_applicable(&self, _rule: &AlertRule, _state: &ActiveQueryState) -> bool {
        // Here we could add logic like "User whitelist/blacklist" if added to rule
        // Currently rule has Region (cluster handles this), DataSource (cluster handles this).
        // So essentially all queries on this cluster are subject to this rule.
        true
    }

    pub async fn test_alert(&self, rule_id: i32) -> ApiResult<()> {
        let rule = self.get_rule(rule_id).await?;
        let dummy_message = format!(
            "🔔 [测试预警] 这是来自 StarRocks Admin 的测试告警消息。\n\n规则名称: {}\n监控项: {}\n状态: 正常连通性测试",
            rule.name, rule.sub_type
        );
        let mentions = rule.receivers.iter().map(|r| r.name.clone()).collect();
        self.send_webhook(&rule, &dummy_message, mentions).await
    }

    pub async fn kill_query(&self, history_id: i32) -> ApiResult<()> {
        // 1. Get History to find connection information
        let history: AlertHistory =
            sqlx::query_as("SELECT * FROM warning_rule.alert_history WHERE id = ?")
                .bind(history_id)
                .fetch_one(&self.pool)
                .await
                .map_err(ApiError::database_error)?;

        let ds_name = history
            .host
            .as_deref()
            .ok_or_else(|| ApiError::invalid_data("History missing data source name"))?;

        let mut connection_id = history.connection_id.clone();
        let mut fe_ip = history.fe_ip.clone();

        // Fallback: If metadata is missing (old record), try to find it in active_queries if it's currently alerting
        if (connection_id.is_none() || fe_ip.is_none())
            && (history.status.as_deref() == Some("Alerting")
                || history.status.as_deref() == Some("Suppressed"))
        {
            tracing::info!(
                "[Alert Service] Metadata missing for h_id {}, searching active_queries for {}",
                history.id,
                history.query_id
            );
            // Search all active queries for this query_id
            for entry in self.active_queries.iter() {
                if entry.value().query_id == history.query_id {
                    connection_id = Some(entry.value().connection_id.clone());
                    fe_ip = Some(entry.value().fe_ip.clone());
                    break;
                }
            }
        }

        let connection_id =
            connection_id.ok_or_else(|| ApiError::invalid_data("History missing connection_id"))?;
        let fe_ip = fe_ip.ok_or_else(|| ApiError::invalid_data("History missing fe_ip"))?;

        let ds: ResourceDataSource =
            sqlx::query_as("SELECT * FROM resource_data_sources WHERE name = ?")
                .bind(ds_name)
                .fetch_optional(&self.pool)
                .await
                .map_err(ApiError::database_error)?
                .ok_or_else(|| ApiError::not_found(format!("Data source {} not found", ds_name)))?;

        let cluster = self.ds_to_cluster(&ds);

        // Check if query is already marked as non-alerting (Resolved or Killed)
        #[allow(clippy::collapsible_if)]
        if let Some(status) = &history.status {
            if status == "Killed" || status == "Resolved" {
                tracing::info!(
                    "[Alert Service] kill_query called for record {} which is already {}",
                    history_id,
                    status
                );
                return Ok(()); // Idempotent success
            }
        }

        let (_target_host, _target_port) = self.resolve_fe_endpoint(&cluster, &fe_ip);

        let mut kill_error = None;
        if let Err(e) = self
            .execute_direct_kill(&cluster, &connection_id, &fe_ip, true)
            .await
        {
            tracing::error!(
                "[Alert Service] Manual KILL QUERY failed for {}: {}",
                history.query_id,
                e
            );
            kill_error = Some(e);
        }

        // VERIFICATION with Retry: If KILL QUERY failed or query is still alive, try KILL connection
        let is_alive = if kill_error.is_none() {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let alive = self
                .verify_query_alive(&cluster, &history.query_id, &fe_ip)
                .await;
            if alive {
                tracing::info!(
                    "[Alert Service] KILL QUERY sent but query {} still alive, retrying with KILL {}",
                    history.query_id,
                    connection_id
                );
                if let Err(re) = self
                    .execute_direct_kill(&cluster, &connection_id, &fe_ip, false)
                    .await
                {
                    tracing::error!(
                        "[Alert Service] Manual KILL retry error for {}: {}",
                        history.query_id,
                        re
                    );
                    kill_error = Some(re);
                    true
                } else {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    self.verify_query_alive(&cluster, &history.query_id, &fe_ip)
                        .await
                }
            } else {
                false
            }
        } else {
            // First attempt (KILL QUERY command) failed, retry with KILL
            tracing::info!(
                "[Alert Service] KILL QUERY command failed, retrying with KILL {}",
                connection_id
            );
            if let Err(re) = self
                .execute_direct_kill(&cluster, &connection_id, &fe_ip, false)
                .await
            {
                tracing::error!(
                    "[Alert Service] Manual KILL fallback error for {}: {}",
                    history.query_id,
                    re
                );
                kill_error = Some(re);
                true
            } else {
                kill_error = None; // Reset error since fallback command succeeded
                tokio::time::sleep(Duration::from_secs(2)).await;
                self.verify_query_alive(&cluster, &history.query_id, &fe_ip)
                    .await
            }
        };

        let (new_status, msg_type) = if is_alive {
            ("KillFailed", "manual_kill_failed")
        } else {
            ("Killed", "manual_kill_success")
        };

        tracing::info!(
            "[Alert Service] Manual kill attempt finished (Result: {}, QueryId: {})",
            new_status,
            history.query_id
        );

        // 3. Update history status
        sqlx::query("UPDATE warning_rule.alert_history SET status = ? WHERE id = ?")
            .bind(new_status)
            .bind(history_id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;

        // 4. Send notification
        if let Ok(rule) = self.get_rule(history.rule_id).await {
            let msg = if msg_type == "manual_kill_failed" {
                let reason = if let Some(e) = kill_error {
                    format!("连接 FE 失败: {}", e)
                } else {
                    "强杀指令已发送，但查询在 2s 后依然存在".to_string()
                };
                self.build_manual_kill_failed_message(&rule, &history, &reason)
            } else {
                self.build_manual_kill_message(&rule, &history)
            };
            let mentions: Vec<String> = rule
                .receivers
                .iter()
                .flat_map(|r| r.email.clone())
                .collect();
            let _ = self.send_webhook(&rule, &msg, mentions).await;
        }

        Ok(())
    }

    async fn verify_query_alive(&self, cluster: &Cluster, query_id: &str, fe_ip: &str) -> bool {
        match self.fetch_full_sql(cluster, query_id, Some(fe_ip)).await {
            Some(sql) => !sql.is_empty(),
            None => false,
        }
    }

    pub async fn kill_query_by_query_id(&self, query_id: &str) -> ApiResult<()> {
        let history = self.get_history_by_query_id(query_id).await?;
        self.kill_query(history.id).await
    }

    pub async fn whitelist_query(&self, history_id: i32) -> ApiResult<()> {
        sqlx::query("UPDATE warning_rule.alert_history SET status = 'Whitelisted' WHERE id = ? AND status IN ('Alerting', 'Suppressed')")
            .bind(history_id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;

        Ok(())
    }

    pub async fn whitelist_query_by_query_id(&self, query_id: &str) -> ApiResult<()> {
        sqlx::query("UPDATE warning_rule.alert_history SET status = 'Whitelisted' WHERE query_id = ? AND status IN ('Alerting', 'KillFailed', 'Suppressed')")
            .bind(query_id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;

        Ok(())
    }

    async fn execute_direct_kill(
        &self,
        cluster: &Cluster,
        connection_id: &str,
        fe_ip: &str,
        is_query_only: bool,
    ) -> ApiResult<()> {
        let (target_host, target_port) = self.resolve_fe_endpoint(cluster, fe_ip);

        let mut target_cluster = cluster.clone();

        // CRITICAL: We must use a unique ID for the pool manager to avoid hitting a cached pool
        // that connects to the wrong host (since mapping redirects standard cluster ID to different hosts).
        let fe_pool_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            cluster.id.hash(&mut hasher);
            target_host.hash(&mut hasher);
            target_port.hash(&mut hasher);
            let h = hasher.finish();
            // Use bitwise trick to ensure it's a stable negative i64 to avoid collision with real cluster IDs
            -((h & 0x7FFFFFFFFFFFFFFF) as i64).max(1)
        };

        target_cluster.id = fe_pool_id;
        target_cluster.fe_host = target_host.clone();
        target_cluster.fe_query_port = target_port;

        let pool = self.mysql_pool_manager.get_pool(&target_cluster).await?;
        let mut conn = pool.get_conn().await.map_err(|e| {
            ApiError::internal_error(format!(
                "Failed to connect to SR FE ({}:{}): {}",
                target_cluster.fe_host, target_cluster.fe_query_port, e
            ))
        })?;

        use mysql_async::prelude::Queryable;
        let kill_sql = if is_query_only {
            format!("KILL QUERY {}", connection_id)
        } else {
            format!("KILL {}", connection_id)
        };

        if let Err(e) = conn.query_drop(&kill_sql).await {
            #[allow(clippy::collapsible_if)]
            if let mysql_async::Error::Server(ref server_err) = e {
                if server_err.code == 1094 {
                    // Error 1094 means query not found on THIS specific FE.
                    // We must NOT return Ok() because it might be running on a different FE due to wrong mapping.
                    let err_msg = format!(
                        "强杀失败：在目标 FE {}:{} 上找不到此queryid。1.查询或已结束。2.请检查 fe_mapping 是否正确映射了原始 IP {}。3.确保没通过 LB 进行映射连接。",
                        target_host, target_port, fe_ip
                    );
                    tracing::warn!(
                        "[Alert Service] Kill target NOT FOUND on connected FE. Target: {}:{}, Original: {}, ConnID: {}, Command: {}",
                        target_host,
                        target_port,
                        fe_ip,
                        connection_id,
                        kill_sql
                    );
                    return Err(ApiError::invalid_data(err_msg));
                }
            }
            return Err(ApiError::internal_error(format!(
                "连接 FE {}:{} 强杀失败 (原始 IP: {}, 指令: {}): {}",
                target_host, target_port, fe_ip, kill_sql, e
            )));
        }

        Ok(())
    }

    async fn fetch_full_sql(
        &self,
        cluster: &Cluster,
        query_id: &str,
        fe_ip: Option<&str>,
    ) -> Option<String> {
        // 1. Try to get SQL from the specific FE via 'show proc'
        #[allow(clippy::collapsible_if)]
        if let Some(ip) = fe_ip {
            if !ip.is_empty() {
                let mut target_cluster = cluster.clone();

                // Use a unique negative ID for caching pools specific to FE IPs
                // This prevents creating a new pool for every SQL fetch.
                let fe_pool_id = {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    cluster.id.hash(&mut hasher);
                    ip.hash(&mut hasher);
                    // Ensure it's negative to stay out of the way of real cluster IDs
                    let h = hasher.finish();
                    -((h & 0x7FFFFFFFFFFFFFFF) as i64).max(1)
                };

                target_cluster.id = fe_pool_id;

                let (target_host, target_port) = self.resolve_fe_endpoint(cluster, ip);
                target_cluster.fe_host = target_host;
                target_cluster.fe_query_port = target_port;

                let mut attempts = 0;
                let max_attempts = 3;
                let mut found_sql = None;

                // Use cluster defined timeout, default to 10s if invalid
                let timeout_secs = if cluster.connection_timeout > 0 {
                    cluster.connection_timeout as u64
                } else {
                    10
                };

                while attempts < max_attempts {
                    attempts += 1;

                    let fe_conn_future = async {
                        if let Ok(pool) = self.mysql_pool_manager.get_pool(&target_cluster).await {
                            if let Ok(mut conn) = pool.get_conn().await {
                                use mysql_async::prelude::Queryable;
                                let show_sql = format!("show proc '/current_queries/{}'", query_id);
                                if let Ok(rows) =
                                    conn.query::<mysql_async::Row, _>(show_sql.clone()).await
                                {
                                    if let Some(row) = rows.into_iter().next() {
                                        let sql: String = row
                                            .get::<Option<String>, &str>("Sql")
                                            .flatten()
                                            .or_else(|| {
                                                row.get::<Option<String>, &str>("Stmt").flatten()
                                            })
                                            .or_else(|| {
                                                row.get::<Option<String>, &str>("Info").flatten()
                                            })
                                            .or_else(|| {
                                                row.get::<Option<String>, usize>(row.len() - 1)
                                                    .flatten()
                                            })
                                            .unwrap_or_default();

                                        if !sql.is_empty() && !sql.contains("...") && sql.len() > 5
                                        {
                                            return Some(sql);
                                        }
                                    }
                                }
                            }
                        }
                        None
                    };

                    // Execute with dynamic timeout
                    match time::timeout(Duration::from_secs(timeout_secs), fe_conn_future).await {
                        Ok(Some(sql)) => {
                            found_sql = Some(sql);
                            break;
                        },
                        Ok(None) => {
                            if self.audit_config.debug {
                                tracing::warn!(
                                    "[Fetch SQL Debug] No data found on FE {} (Attempt {}/{})",
                                    ip,
                                    attempts,
                                    max_attempts
                                );
                            }
                        },
                        Err(_) => {
                            tracing::warn!(
                                "[Fetch SQL] Timeout ({}s) attempting to connect to FE {} for query {} (Attempt {}/{})",
                                timeout_secs,
                                ip,
                                query_id,
                                attempts,
                                max_attempts
                            );
                        },
                    }

                    if attempts < max_attempts {
                        // Small delay before retry
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                    }
                }

                if let Some(sql) = found_sql {
                    return Some(sql);
                }
            }
        }

        // 2. Fallback: try Audit Log
        self.fetch_from_audit_log(cluster, query_id).await
    }

    async fn fetch_from_audit_log(&self, cluster: &Cluster, query_id: &str) -> Option<String> {
        let pool = match self.mysql_pool_manager.get_pool(cluster).await {
            Ok(p) => p,
            Err(_) => return None,
        };
        let mut conn = match pool.get_conn().await {
            Ok(c) => c,
            Err(_) => return None,
        };

        use mysql_async::prelude::Queryable;

        let audit_table = self.audit_config.full_table_name();
        let audit_queries = [
            format!("SELECT stmt FROM {} WHERE queryId = ? LIMIT 1", audit_table),
            format!("SELECT stmt FROM {} WHERE query_id = ? LIMIT 1", audit_table),
        ];

        for audit_sql in audit_queries {
            #[allow(clippy::collapsible_if)]
            if let Ok(rows) = conn
                .exec::<mysql_async::Row, _, _>(audit_sql, (query_id,))
                .await
            {
                if let Some(row) = rows.into_iter().next() {
                    let stmt: String = row.get(0).unwrap_or_default();
                    if !stmt.is_empty() {
                        return Some(stmt);
                    }
                }
            }
        }

        tracing::warn!(
            "[Fetch SQL] Audit Log lookup failed for query {} in cluster {}",
            query_id,
            cluster.name
        );
        None
    }

    fn translate_sub_type(&self, sub_type: &str) -> String {
        match sub_type {
            "Memory" => "内存使用",
            "Cpu" | "CPUTime" => "CPU时间",
            "ScanRows" => "扫描行数",
            "ExecutionTime" => "执行时间",
            _ => sub_type,
        }
        .to_string()
    }

    fn check_violation(&self, rule: &AlertRule, state: &ActiveQueryState) -> bool {
        // SubType: Memory (GB), CPU (s), Scan Rows (Count), Execution Time (s)
        let is_violated = match rule.sub_type.as_str() {
            "Memory" => state.memory_usage > rule.threshold * 1024 * 1024 * 1024,
            "Cpu" | "CPUTime" => state.cpu_time > rule.threshold as f64,
            "ExecutionTime" => state.exec_time > rule.threshold as f64,
            "ScanRows" => state.scan_rows > rule.threshold,
            _ => false,
        };

        if self.audit_config.debug && is_violated {
            tracing::info!(
                "[Alert Monitor Debug] VIOLATION DETECTED - Rule: {}, QueryId: {}, SubType: {}, Value: {}, Threshold: {}",
                rule.name,
                state.query_id,
                rule.sub_type,
                self.get_current_value(rule, state),
                rule.threshold
            );
        }

        is_violated
    }

    async fn process_alert(
        &self,
        cluster: &Cluster,
        rule: &AlertRule,
        state: &mut ActiveQueryState,
    ) {
        let now = Utc::now();

        // 1. Initialize first_alert_time and check for whitelist if needed (recover from DB)
        // 1. Initialize first_alert_time and check for whitelist if needed
        let mut is_whitelisted = false;

        // Check if query is whitelisted in DB
        let db_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM warning_rule.alert_history WHERE query_id = ? AND status = 'Whitelisted' LIMIT 1"
        )
        .bind(&state.query_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        if db_status.as_deref() == Some("Whitelisted") {
            is_whitelisted = true;
        }

        if state.first_alert_time.is_none() {
            // First time alerting in this session, recover start time and counts from DB if exists
            #[allow(clippy::type_complexity)]
            let recovered: Option<(Option<DateTime<Utc>>, Option<i32>, DateTime<Utc>)> = sqlx::query_as(
                "SELECT last_alert_time, alert_count, created_at FROM warning_rule.alert_history WHERE query_id = ? AND status IN ('Alerting', 'Whitelisted', 'Suppressed') ORDER BY id DESC LIMIT 1"
            )
            .bind(&state.query_id)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            if let Some((last_a_time, a_count, created_at)) = recovered {
                state.first_alert_time = Some(created_at);

                // Recover basic interval state to prevent spamming
                if let Some(last) = last_a_time {
                    // Initialize all configured channels with the last recorded alert time
                    // This ensures we respect the interval if the state was lost mid-alerting
                    if let Some(ref channels) = rule.channels {
                        for ch in channels {
                            let key = (rule.id, ch.r#type.clone());
                            state.channel_alert_times.entry(key).or_insert(last);
                        }
                    } else {
                        let legacy_key =
                            (rule.id, rule.channel.clone().unwrap_or_else(|| "tv".to_string()));
                        state.channel_alert_times.entry(legacy_key).or_insert(last);
                    }
                }

                if let Some(count) = a_count {
                    // Distribute count evenly across channels or just set as base
                    if let Some(ref channels) = rule.channels {
                        for ch in channels {
                            let key = (rule.id, ch.r#type.clone());
                            state.channel_alert_counts.entry(key).or_insert(count);
                        }
                    } else {
                        let legacy_key =
                            (rule.id, rule.channel.clone().unwrap_or_else(|| "tv".to_string()));
                        state
                            .channel_alert_counts
                            .entry(legacy_key)
                            .or_insert(count);
                    }
                }
            } else {
                state.first_alert_time = Some(now);
            }
        }

        let duration = now - state.first_alert_time.unwrap();

        // 2. Determine Notification Intervals and Active Channels
        let current_hm = Local::now().format("%H:%M").to_string();
        let mut active_channels = Vec::new();
        if let Some(ref channels) = rule.channels {
            if !channels.is_empty() {
                for ch in channels {
                    if self.check_time_range(&current_hm, &ch.start_time, &ch.end_time) {
                        active_channels.push(ch.clone());
                    }
                }
            } else {
                active_channels.push(self.convert_legacy_channel(rule));
            }
        } else {
            active_channels.push(self.convert_legacy_channel(rule));
        }
        let is_off_duty = active_channels.is_empty();

        // 3. Auto-Kill Check (Independent of channel intervals)
        let mut is_auto_killed = false;
        let mut auto_kill_failed = false;
        let mut kill_error = None;
        if rule.auto_kill {
            if is_whitelisted {
                if self.audit_config.debug {
                    tracing::info!(
                        "[Alert Monitor Debug] Skipping auto-kill for query {} (whitelisted)",
                        state.query_id
                    );
                }
            } else if let Some(threshold_mins) = rule.auto_kill_threshold_minutes {
                #[allow(clippy::collapsible_if)]
                if duration >= ChronoDuration::minutes(threshold_mins as i64) {
                    let mut first_attempt_error = None;
                    if let Err(e) = self
                        .execute_direct_kill(cluster, &state.connection_id, &state.fe_ip, true)
                        .await
                    {
                        first_attempt_error = Some(e);
                    }

                    let is_alive = if first_attempt_error.is_none() {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        if self
                            .verify_query_alive(cluster, &state.query_id, &state.fe_ip)
                            .await
                        {
                            tracing::info!(
                                "[Auto Kill] KILL QUERY sent but query {} still alive, retrying with KILL {}",
                                state.query_id,
                                state.connection_id
                            );
                            if let Err(re) = self
                                .execute_direct_kill(
                                    cluster,
                                    &state.connection_id,
                                    &state.fe_ip,
                                    false,
                                )
                                .await
                            {
                                kill_error = Some(re.to_string());
                                true
                            } else {
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                self.verify_query_alive(cluster, &state.query_id, &state.fe_ip)
                                    .await
                            }
                        } else {
                            false
                        }
                    } else {
                        tracing::info!(
                            "[Auto Kill ] KILL QUERY command failed, retrying with KILL {}",
                            state.connection_id
                        );
                        if let Err(re) = self
                            .execute_direct_kill(cluster, &state.connection_id, &state.fe_ip, false)
                            .await
                        {
                            kill_error = Some(re.to_string());
                            true
                        } else {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            self.verify_query_alive(cluster, &state.query_id, &state.fe_ip)
                                .await
                        }
                    };

                    if is_alive {
                        auto_kill_failed = true;
                        let (target_host, target_port) =
                            self.resolve_fe_endpoint(cluster, &state.fe_ip);
                        tracing::warn!(
                            "[Auto Kill Failed] Query {} still alive after both KILL QUERY and KILL commands. Target: {}:{}",
                            state.query_id,
                            target_host,
                            target_port
                        );
                    } else {
                        is_auto_killed = true;
                        tracing::info!(
                            "[Auto Kill Success] Query {} killed by rule {}.",
                            state.query_id,
                            rule.name
                        );
                    }
                }
            }
        }

        // 4. Fetch Full SQL if needed
        // We'll fetch it if ANY channel might notify or if it's an auto-kill event
        let mut might_notify = is_auto_killed || auto_kill_failed;
        if !might_notify {
            for ch in &active_channels {
                let channel_key = (rule.id, ch.r#type.clone());
                let interval = ch
                    .notify_interval_minutes
                    .unwrap_or(rule.notify_interval_minutes) as i64;
                let last = state.channel_alert_times.get(&channel_key);
                if last.is_none() || (now - *last.unwrap()).num_seconds() >= interval * 60 {
                    might_notify = true;
                    break;
                }
            }
        }

        if might_notify {
            #[allow(clippy::collapsible_if)]
            if state.sql_text.is_empty()
                || state.sql_text.contains("...")
                || state.sql_text.len() < 100
            {
                if let Some(full_sql) = self
                    .fetch_full_sql(cluster, &state.query_id, Some(&state.fe_ip))
                    .await
                {
                    state.sql_text = full_sql;
                }
            }
        }

        // 5. Send Notifications per Channel
        let mut ivr_msg_ids: Option<Vec<String>> = None;
        let mut any_notified = false;
        let mut final_alert_count = 0;

        for ch in active_channels {
            let channel_key = (rule.id, ch.r#type.clone());
            let channel_interval_mins =
                ch.notify_interval_minutes
                    .unwrap_or(rule.notify_interval_minutes) as i64;

            let last_channel_time = state.channel_alert_times.get(&channel_key).cloned();
            let should_notify_channel = match last_channel_time {
                None => true,
                Some(last) => (now - last).num_seconds() >= channel_interval_mins * 60,
            };

            let mut force_notify =
                (is_auto_killed || auto_kill_failed) && !state.auto_kill_notified;
            if force_notify && auto_kill_failed {
                // Check if already notified KillFailed in DB record to prevent spam across scan cycles
                let already_failed = sqlx::query_scalar::<_, i32>("SELECT 1 FROM warning_rule.alert_history WHERE query_id = ? AND status = 'KillFailed' LIMIT 1")
                   .bind(&state.query_id)
                   .fetch_optional(&self.pool)
                   .await
                   .unwrap_or(None)
                   .is_some();
                if already_failed && auto_kill_failed {
                    force_notify = false;
                }
            }

            if (should_notify_channel || force_notify) && !is_whitelisted {
                // Determine if manager should be notified for this channel
                let duration = now - state.first_alert_time.unwrap();
                // Escalate to manager if duration > 2x interval OR if it's a critical Kill event
                let notify_manager = duration >= ChronoDuration::minutes(2 * channel_interval_mins)
                    || is_auto_killed
                    || auto_kill_failed;

                if self.audit_config.debug {
                    tracing::info!(
                        "[Alert Debug] Triggering alert for Rule '{}' (ID: {}). Channel: {}, Interval: {}m, AutoKill: {}, Manager: {}",
                        rule.name,
                        rule.id,
                        ch.r#type,
                        channel_interval_mins,
                        is_auto_killed,
                        notify_manager
                    );
                }

                state.channel_alert_times.insert(channel_key.clone(), now);
                let count_ref = state.channel_alert_counts.entry(channel_key).or_insert(0);
                *count_ref += 1;
                final_alert_count = final_alert_count.max(*count_ref);
                any_notified = true;

                if is_auto_killed || auto_kill_failed {
                    state.auto_kill_notified = true;
                }

                let target_receivers: Vec<&AlertReceiver> = rule
                    .receivers
                    .iter()
                    .filter(|r| if notify_manager { true } else { r.role != "manager" })
                    .collect();

                let message = if is_auto_killed {
                    self.build_auto_kill_message(rule, state)
                } else if auto_kill_failed {
                    let reason = if let Some(ref e) = kill_error {
                        format!("连接 FE 失败: {}", e)
                    } else {
                        "强杀指令已发送，但查询在 2s 后依然存在".to_string()
                    };
                    self.build_auto_kill_failed_message(rule, state, &reason)
                } else {
                    self.build_alert_message(rule, state)
                };

                if ch.r#type == "ivr" {
                    if is_auto_killed {
                        tracing::info!(
                            "[IVR Skipped] Rule: {}, Reason: Kill notification suppressed for IVR",
                            rule.name
                        );
                        continue;
                    }
                    match self
                        .send_ivr_channel_alert(rule, &ch, &target_receivers)
                        .await
                    {
                        Ok(ids) => {
                            if !ids.is_empty() {
                                if ivr_msg_ids.is_none() {
                                    ivr_msg_ids = Some(Vec::new());
                                }
                                if let Some(ref mut vec) = ivr_msg_ids {
                                    vec.extend(ids);
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("[Alert Failed] IVR Rule: {}, Error: {}", rule.name, e)
                        },
                    }
                } else {
                    // TV
                    let bot_id = ch
                        .template_id
                        .as_deref()
                        .or(rule.template_id.as_deref())
                        .unwrap_or("");
                    if !bot_id.is_empty() {
                        let mentions: Vec<String> = target_receivers
                            .iter()
                            .flat_map(|r| r.email.clone())
                            .collect();
                        let _ = self.send_notification(bot_id, &message, mentions).await;
                    }
                }
            }
        }

        // 6. Persist to History
        let status = if is_auto_killed {
            "Killed".to_string()
        } else if auto_kill_failed {
            "KillFailed".to_string()
        } else if is_whitelisted {
            "Whitelisted".to_string()
        } else if is_off_duty {
            "Suppressed".to_string()
        } else {
            "Alerting".to_string()
        };

        let threshold_str = self.format_threshold(rule);
        let history = AlertHistory {
            id: 0,
            rule_id: rule.id,
            query_id: state.query_id.clone(),
            start_time: Some(state.start_time.clone()),
            user: Some(state.user.clone()),
            host: Some(state.datasource_name.clone()),
            db: Some(state.db.clone()),
            department: None,
            sql_text: Some(state.sql_text.clone()),
            violation_detail: Some(format!(
                "{}>{}",
                self.translate_sub_type(&rule.sub_type),
                threshold_str
            )),
            status: Some(status),

            // Only update counts if we just notified or was auto-killed (and not whitelisted)
            alert_count: if any_notified { Some(final_alert_count) } else { None },
            last_alert_time: if any_notified { Some(now) } else { None },

            cpu_time: Some(state.cpu_time),
            mem_usage: Some(state.memory_usage),
            exec_time: Some(state.exec_time),
            scan_rows: Some(state.scan_rows),
            scan_bytes: Some(state.scan_bytes),
            connection_id: Some(state.connection_id.clone()),
            fe_ip: Some(state.fe_ip.clone()),
            created_at: now,
            remark: None,
            repair_person: None,
            ivr_msg_id: ivr_msg_ids.map(|ids: Vec<String>| ids.join(",")),
        };

        // Need to pass history... (save_history handles final_dept logic)

        if let Err(e) = self.save_history(history).await {
            tracing::error!(
                "[Alert Monitor] Failed to save history for query {}: {}",
                state.query_id,
                e
            );
        } else if self.audit_config.debug {
            tracing::info!(
                "[Alert Monitor Debug] Successfully saved history for query: {}",
                state.query_id
            );
        }
    }

    fn build_alert_message(&self, rule: &AlertRule, state: &ActiveQueryState) -> String {
        let sub_type_cn = self.translate_sub_type(&rule.sub_type);
        let current_val = self.get_current_value(rule, state);
        let threshold = self.format_threshold(rule);

        // SQL link - target internal dashboard
        let sql_link = format!("http://example.com/share/sql/{}", state.query_id);

        format!(
            "🚨 StarRocks {}告警\n\
            集群: {}\n\
            告警原因: {}: {} > {}\n\
            查询详情:\n\
            • 开始时间: {}\n\
            • 查询ID: {}\n\
            • 连接ID: {}\n\
            • 数据库: {}\n\
            • 用户: {}\n\
            • 扫描字节: {}\n\
            • 扫描行数: {}\n\
            • 内存使用: {}\n\
            • CPU时间: {} s\n\
            • 执行时间: {} s\n\
            • SQL详情: {}",
            sub_type_cn,
            state.datasource_name,
            sub_type_cn,
            current_val,
            threshold,
            state.start_time,
            state.query_id,
            state.connection_id,
            state.db,
            state.user,
            format_bytes(state.scan_bytes),
            state.scan_rows,
            format_bytes(state.memory_usage),
            state.cpu_time,
            state.exec_time,
            sql_link
        )
    }

    fn build_auto_kill_message(&self, rule: &AlertRule, state: &ActiveQueryState) -> String {
        let sub_type_cn = self.translate_sub_type(&rule.sub_type);
        let current_val = self.get_current_value(rule, state);
        let threshold = self.format_threshold(rule);
        let sql_link = format!("http://example.com/share/sql/{}", state.query_id);

        format!(
            "🛡️ 异常sql自动kill成功\n\
            集群: {}\n\
            告警原因: {}: {} > {}\n\
            查询详情:\n\
            • 开始时间: {}\n\
            • 查询ID: {}\n\
            • 连接ID: {}\n\
            • 数据库: {}\n\
            • 用户: {}\n\
            • 扫描字节: {}\n\
            • 扫描行数: {}\n\
            • 内存使用: {}\n\
            • CPU时间: {} s\n\
            • 执行时间: {} s\n\
            • SQL详情: {}",
            state.datasource_name,
            sub_type_cn,
            current_val,
            threshold,
            state.start_time,
            state.query_id,
            state.connection_id,
            state.db,
            state.user,
            format_bytes(state.scan_bytes),
            state.scan_rows,
            format_bytes(state.memory_usage),
            state.cpu_time,
            state.exec_time,
            sql_link
        )
    }

    fn build_auto_kill_failed_message(
        &self,
        rule: &AlertRule,
        state: &ActiveQueryState,
        reason: &str,
    ) -> String {
        let sub_type_cn = self.translate_sub_type(&rule.sub_type);
        let current_val = self.get_current_value(rule, state);
        let threshold = self.format_threshold(rule);
        let sql_link = format!("http://example.com/share/sql/{}", state.query_id);

        format!(
            "⚠️ 异常sql自动kill失败\n\
            集群: {}\n\
            失败原因: {}\n\
            告警原因: {}: {} > {}\n\
            查询详情:\n\
            • 开始时间: {}\n\
            • 查询ID: {}\n\
            • 连接ID: {}\n\
            • 数据库: {}\n\
            • 用户: {}\n\
            • 扫描字节: {}\n\
            • 扫描行数: {}\n\
            • 内存使用: {}\n\
            • CPU时间: {} s\n\
            • 执行时间: {} s\n\
            • SQL详情: {}",
            state.datasource_name,
            reason,
            sub_type_cn,
            current_val,
            threshold,
            state.start_time,
            state.query_id,
            state.connection_id,
            state.db,
            state.user,
            format_bytes(state.scan_bytes),
            state.scan_rows,
            format_bytes(state.memory_usage),
            state.cpu_time,
            state.exec_time,
            sql_link
        )
    }

    fn build_manual_kill_message(&self, rule: &AlertRule, h: &AlertHistory) -> String {
        let sql_link = format!("http://example.com/share/sql/{}", h.query_id);
        format!(
            "🛡️ 异常sql手动kill成功\n\
            集群: {}\n\
            告警规则: {}\n\
            告警原因: {}\n\
            查询详情:\n\
            • 开始时间: {}\n\
            • 查询ID: {}\n\
            • 连接ID: {}\n\
            • 数据库: {}\n\
            • 用户: {}\n\
            • 扫描字节: {}\n\
            • 扫描行数: {}\n\
            • 内存使用: {}\n\
            • CPU时间: {} s\n\
            • 执行时间: {} s\n\
            • SQL详情: {}",
            h.host.as_deref().unwrap_or("Unknown"),
            rule.name,
            h.violation_detail.as_deref().unwrap_or("Unknown"),
            h.start_time
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            h.query_id,
            h.connection_id.as_deref().unwrap_or("N/A"),
            h.db.as_deref().unwrap_or("N/A"),
            h.user.as_deref().unwrap_or("N/A"),
            format_bytes(h.scan_bytes.unwrap_or(0)),
            h.scan_rows.unwrap_or(0),
            format_bytes(h.mem_usage.unwrap_or(0)),
            h.cpu_time.unwrap_or(0.0),
            h.exec_time.unwrap_or(0.0),
            sql_link
        )
    }

    fn build_manual_kill_failed_message(
        &self,
        rule: &AlertRule,
        h: &AlertHistory,
        reason: &str,
    ) -> String {
        let sql_link = format!("http://example.com/share/sql/{}", h.query_id);
        format!(
            "⚠️ 异常sql手动kill失败\n\
            集群: {}\n\
            失败原因: {}\n\
            告警规则: {}\n\
            告警原因: {}\n\
            查询详情:\n\
            • 开始时间: {}\n\
            • 查询ID: {}\n\
            • 连接ID: {}\n\
            • 数据库: {}\n\
            • 用户: {}\n\
            • 扫描字节: {}\n\
            • 扫描行数: {}\n\
            • 内存使用: {}\n\
            • CPU时间: {} s\n\
            • 执行时间: {} s\n\
            • SQL详情: {}",
            h.host.as_deref().unwrap_or("Unknown"),
            reason,
            rule.name,
            h.violation_detail.as_deref().unwrap_or("Unknown"),
            h.start_time
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            h.query_id,
            h.connection_id.as_deref().unwrap_or("N/A"),
            h.db.as_deref().unwrap_or("N/A"),
            h.user.as_deref().unwrap_or("N/A"),
            format_bytes(h.scan_bytes.unwrap_or(0)),
            h.scan_rows.unwrap_or(0),
            format_bytes(h.mem_usage.unwrap_or(0)),
            h.cpu_time.unwrap_or(0.0),
            h.exec_time.unwrap_or(0.0),
            sql_link
        )
    }

    fn format_threshold(&self, rule: &AlertRule) -> String {
        match rule.sub_type.as_str() {
            "Memory" => format!("{} GB", rule.threshold),
            "ExecutionTime" => format!("{} s", rule.threshold),
            "Cpu" | "CPUTime" => format!("{} s", rule.threshold),
            _ => format!("{}", rule.threshold),
        }
    }

    fn get_current_value(&self, rule: &AlertRule, state: &ActiveQueryState) -> String {
        match rule.sub_type.as_str() {
            "Memory" => format_bytes(state.memory_usage),
            "ExecutionTime" => format!("{:.2} s", state.exec_time),
            "ScanRows" => format!("{}", state.scan_rows),
            "Cpu" | "CPUTime" => format!("{:.2} s", state.cpu_time),
            _ => "?".to_string(),
        }
    }

    pub async fn send_notification(
        &self,
        bot_id: &str,
        message: &str,
        mentions: Vec<String>,
    ) -> ApiResult<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| {
                ApiError::internal_error(format!("Failed to build webhook client: {}", e))
            })?;
        let payload = json!({
            "botId": bot_id,
            "message": message,
            "mentions": mentions
        });

        let url = "https://example.com/alert/v2/array";

        let _ = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::internal_error(format!("Webhook failed: {}", e)))?;

        Ok(())
    }

    async fn resolve_receiver_phones(&self, receivers: &[&AlertReceiver]) -> Vec<String> {
        let mut phones = Vec::new();
        // Dedup phones
        let mut seen = std::collections::HashSet::new();

        for r in receivers {
            // Query exact match on name or user or partially
            // Using exact match on name/user as requested "fill name automatically appear phone number"
            let p: Option<String> = sqlx::query_scalar("SELECT phone FROM basic_information.employee WHERE (name = ? OR user = ?) AND phone != '' LIMIT 1")
                 .bind(&r.name)
                 .bind(&r.name)
                 .fetch_optional(&self.pool)
                 .await
                 .unwrap_or(None);
            #[allow(clippy::collapsible_if)]
            if let Some(s) = p {
                if !s.is_empty() && seen.insert(s.clone()) {
                    phones.push(s);
                }
            }
        }
        phones
    }

    async fn send_ivr_channel_alert(
        &self,
        rule: &AlertRule,
        channel: &AlertChannel,
        receivers: &[&AlertReceiver],
    ) -> ApiResult<Vec<String>> {
        let phones = self.resolve_receiver_phones(receivers).await;
        if phones.is_empty() {
            tracing::warn!("No phones found for IVR alert rule: {}", rule.name);
            return Ok(vec![]);
        }
        let mobile = phones.join(",");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
        // Update to test environment as requested/verified by user
        let url = "https://example/msg/ivrAlarm";

        // Use configured ivr_params from channel
        let params = channel.ivr_params.clone().unwrap_or(json!({}));

        // Ensure template is present
        let template = channel.ivr_template.as_deref().unwrap_or("");

        // Use configured secret or default
        let secret = channel.ivr_secret.as_deref().unwrap_or("695B539ADBDE6");

        let payload = json!({
            "secret": secret,
            "mobile": mobile,
            "template": template,
            "templateParams": params
        });

        let resp = client
            .post(url)
            .header("User-Agent", "curl/7.64.1") // Mimic curl
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::internal_error(format!("IVR Req failed: {}", e)))?;
        let resp_text = resp
            .text()
            .await
            .map_err(|e| ApiError::internal_error(format!("IVR text failed: {}", e)))?;
        tracing::info!("[IVR Debug] Raw Response: {}", resp_text);

        let resp_json: serde_json::Value = serde_json::from_str(&resp_text).map_err(|e| {
            ApiError::internal_error(format!("IVR JSON failed: {}, Raw: {}", e, resp_text))
        })?;

        // resp_json: {"code":0, "message":"OK", "data":{"msgId":["..."]}}
        if resp_json["code"].as_i64() == Some(0) {
            let ids = resp_json["data"]["msgId"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            tracing::info!(
                "[IVR Alert Sent] Rule: {}, Phones: {}, MsgIds: {:?}",
                rule.name,
                mobile,
                ids
            );
            Ok(ids)
        } else {
            tracing::error!("IVR Alert Failed Response: {:?}", resp_json);
            // "failed need to print log" -> Done.
            Err(ApiError::internal_error(format!("IVR API Error: {:?}", resp_json)))
        }
    }

    // Kept for backward compat if needed, but unused if switched to channel logic
    #[allow(dead_code)]
    async fn send_ivr_alert(
        &self,
        rule: &AlertRule,
        receivers: &[&AlertReceiver],
    ) -> ApiResult<Vec<String>> {
        // Construct a temp channel from rule fields
        let ch = self.convert_legacy_channel(rule);
        self.send_ivr_channel_alert(rule, &ch, receivers).await
    }

    fn check_time_range(&self, current_hm: &str, start: &str, end: &str) -> bool {
        // Simple string comparison works for HH:MM format
        // Handle cross-day? No requirement yet, user said "00:00 - 24:00" implying simple range.
        if start <= end {
            current_hm >= start && current_hm <= end
        } else {
            // Cross day: e.g. 23:00 to 06:00
            current_hm >= start || current_hm <= end
        }
    }

    fn convert_legacy_channel(&self, rule: &AlertRule) -> AlertChannel {
        let t = rule.channel.clone().unwrap_or("tv".to_string());
        AlertChannel {
            r#type: t,
            start_time: "00:00".to_string(),
            end_time: "24:00".to_string(),
            template_id: rule.template_id.clone(),
            ivr_template: rule.ivr_template.clone(),
            ivr_secret: rule.ivr_secret.clone(),
            ivr_params: rule.ivr_params.clone(),
            notify_interval_minutes: Some(rule.notify_interval_minutes),
        }
    }

    async fn send_webhook(
        &self,
        rule: &AlertRule,
        message: &str,
        mentions: Vec<String>,
    ) -> ApiResult<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| {
                ApiError::internal_error(format!("Failed to build webhook client: {}", e))
            })?;
        let bot_id = rule
            .channels
            .as_ref()
            .and_then(|c| c.iter().find(|ch| ch.r#type == "tv"))
            .and_then(|ch| ch.template_id.clone())
            .or_else(|| rule.template_id.clone())
            .unwrap_or_else(|| "default_bot_id".into());

        let payload = json!({
            "botId": bot_id,
            "message": message,
            "mentions": mentions
        });

        let url = "https://example.com/alert/v2/array";

        let _ = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::internal_error(format!("Webhook failed: {}", e)))?;

        Ok(())
    }

    // --- DB Operations ---

    async fn get_enabled_rules(&self) -> ApiResult<Vec<AlertRule>> {
        sqlx::query_as("SELECT * FROM warning_rule.alert_rules WHERE enabled = 1")
            .fetch_all(&self.pool)
            .await
            .map_err(ApiError::database_error)
    }

    pub async fn save_history(&self, h: AlertHistory) -> ApiResult<()> {
        // Check if there is an existing record for this query_id
        // Inclued 'Resolved' status to prevent duplicate alert records for same query id
        let existing: Option<(i32, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, sql_text, status FROM warning_rule.alert_history WHERE query_id = ? AND status IN ('Alerting', 'Whitelisted', 'KillFailed', 'Suppressed', 'Resolved') LIMIT 1"
    )
        .bind(&h.query_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        // Lookup department if missing
        let mut final_dept = h.department.clone();
        #[allow(clippy::collapsible_if)]
        if final_dept.is_none() {
            if let Some(user_val) = &h.user {
                // Try to find in employee table first as it's the primary source
                let clean_user = user_val.trim();

                // 1. Poll employee table
                let dept_lookup: Option<String> = sqlx::query_scalar(
                    "SELECT orgName FROM basic_information.employee 
                     WHERE (user = ? OR userId = ?) 
                     AND orgName IS NOT NULL AND orgName != ''
                     LIMIT 1",
                )
                .bind(clean_user)
                .bind(clean_user)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);

                if dept_lookup.is_some() {
                    final_dept = dept_lookup;
                } else {
                    // 2. Poll user_department_dim table (Qualify with basic_information schema)
                    let dim_lookup: Option<String> = sqlx::query_scalar(
                        "SELECT department FROM basic_information.user_department_dim 
                         WHERE user = ? 
                         AND department IS NOT NULL AND department != ''
                         LIMIT 1",
                    )
                    .bind(clean_user)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap_or(None);
                    final_dept = dim_lookup;
                }

                if final_dept.is_none() {
                    tracing::warn!(
                        "[Alert Monitor] Dept lookup failed (Exact Match) for user: {}",
                        clean_user
                    );
                }
            }
        }

        if let Some((id, existing_sql, existing_status)) = existing {
            // Update existing record
            // If the record is already in a terminal/specified status, we might want to preserve it or update it.
            // Requirement: "后续sql状态继续更新" - if status is KillFailed, it stays KillFailed but metrics update.
            let final_status = if let Some(s) = existing_status {
                if s == "Whitelisted" {
                    Some("Whitelisted".to_string())
                } else if s == "KillFailed" {
                    Some("KillFailed".to_string())
                } else {
                    h.status
                }
            } else {
                h.status
            };

            // Only update SQL if the new one is significantly longer or if existing is missing
            let mut final_sql = h.sql_text.clone();
            #[allow(clippy::collapsible_if)]
            if let Some(e_sql) = existing_sql {
                if !e_sql.is_empty()
                    && (h.sql_text.is_none()
                        || h.sql_text.as_ref().map_or(0, |s| s.len()) < e_sql.len())
                {
                    final_sql = Some(e_sql);
                }
            }

            sqlx::query(
                "UPDATE warning_rule.alert_history SET 
                rule_id = ?,
                alert_count = COALESCE(?, alert_count), 
                last_alert_time = COALESCE(?, last_alert_time), 
                cpu_time = ?, mem_usage = ?, exec_time = ?, scan_rows = ?, scan_bytes = ?, violation_detail = ?, sql_text = ?, status = COALESCE(?, status),
                department = COALESCE(?, department), ivr_msg_id = COALESCE(?, ivr_msg_id)
                WHERE id = ?"
            )
            .bind(h.rule_id)
            .bind(h.alert_count)
            .bind(h.last_alert_time)
            .bind(h.cpu_time)
            .bind(h.mem_usage)
            .bind(h.exec_time)
            .bind(h.scan_rows)
            .bind(h.scan_bytes)
            .bind(h.violation_detail)
            .bind(final_sql)
            .bind(final_status)
            .bind(final_dept)
            .bind(h.ivr_msg_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;
        } else {
            // Insert new record
            sqlx::query(
                "INSERT INTO warning_rule.alert_history 
                (rule_id, query_id, start_time, user, host, `db`, department, sql_text, violation_detail, status, alert_count, last_alert_time, cpu_time, mem_usage, exec_time, scan_rows, scan_bytes, connection_id, fe_ip, ivr_msg_id)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(h.rule_id)
            .bind(&h.query_id)
            .bind(h.start_time)
            .bind(&h.user)
            .bind(&h.host)
            .bind(&h.db)
            .bind(final_dept)
            .bind(&h.sql_text)
            .bind(&h.violation_detail)
            .bind(&h.status)
            .bind(h.alert_count)
            .bind(h.last_alert_time)
            .bind(h.cpu_time)
            .bind(h.mem_usage)
            .bind(h.exec_time)
            .bind(h.scan_rows)
            .bind(h.scan_bytes)
            .bind(&h.connection_id)
            .bind(&h.fe_ip)
            .bind(&h.ivr_msg_id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;
        }
        Ok(())
    }

    // --- CRUD for Rules ---
    pub async fn list_rules(&self) -> ApiResult<Vec<AlertRule>> {
        sqlx::query_as("SELECT * FROM warning_rule.alert_rules ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(ApiError::database_error)
    }

    pub async fn create_rule(&self, mut req: CreateAlertRuleRequest) -> ApiResult<AlertRule> {
        // Enforce minimum 3 minutes if IVR channel exists
        if req
            .channels
            .as_ref()
            .is_some_and(|cs| cs.iter().any(|c| c.r#type == "ivr"))
            && req.notify_interval_minutes.unwrap_or(5) < 3
        {
            req.notify_interval_minutes = Some(3);
        }
        let result = sqlx::query(
            "INSERT INTO warning_rule.alert_rules 
            (name, region, tags, data_source, datasource_id, alert_type, sub_type, threshold, starrocks_version, template_id, receivers, enabled, auto_kill, auto_kill_threshold_minutes, notify_interval_minutes, channel, ivr_template, ivr_params, ivr_secret, channels) 
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.region)
        .bind(&req.tags)
        .bind(&req.data_source)
        .bind(req.datasource_id)
        .bind(req.alert_type.unwrap_or("Abnormal SQL".into()))
        .bind(&req.sub_type)
        .bind(req.threshold)
        .bind(&req.starrocks_version)
        .bind(&req.template_id)
        .bind(serde_json::to_value(&req.receivers).unwrap())
        .bind(req.enabled)
        .bind(req.auto_kill.unwrap_or(false))
        .bind(req.auto_kill_threshold_minutes)
        .bind(req.notify_interval_minutes.unwrap_or(5))
        .bind(req.channel.unwrap_or("tv".to_string()))
        .bind(req.ivr_template)
        .bind(req.ivr_params.unwrap_or(json!({})))
        .bind(req.ivr_secret.unwrap_or("695B539ADBDE6".to_string()))
        .bind(serde_json::to_value(req.channels.unwrap_or_default()).unwrap())
        .execute(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        let id = result.last_insert_id() as i32;

        // Immediate check for the new rule
        let _ = self.check_all_clusters().await;

        self.get_rule(id).await
    }

    pub async fn get_rule(&self, id: i32) -> ApiResult<AlertRule> {
        sqlx::query_as("SELECT * FROM warning_rule.alert_rules WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(ApiError::database_error)?
            .ok_or_else(|| ApiError::not_found("Rule not found"))
    }

    pub async fn update_rule(&self, id: i32, req: UpdateAlertRuleRequest) -> ApiResult<AlertRule> {
        // Simpler approach: update each field if Some
        // Or construct SQL.
        // Using sqlx::query with many Option binds is annoying.
        // I will use a simple "SET col = COALESCE(?, col)" pattern? No, that requires passing None if not updating.

        let rule = self.get_rule(id).await?; // Verify exists

        // We will use a series of IFs to build the query string and bind specific values.
        // This is verbose in Rust without a builder.
        // I'll assume users update "all or nothing" from UI often, but for API flexibility, partial updates are good.

        let mut new_notify_interval = req
            .notify_interval_minutes
            .unwrap_or(rule.notify_interval_minutes);

        // Enforce minimum 3 minutes if IVR channel exists
        let has_ivr = req
            .channels
            .as_ref()
            .is_some_and(|cs| cs.iter().any(|c| c.r#type == "ivr"))
            || (req.channels.is_none()
                && rule
                    .channels
                    .as_ref()
                    .is_some_and(|cs| cs.iter().any(|c| c.r#type == "ivr")));
        if has_ivr && new_notify_interval < 3 {
            new_notify_interval = 3;
        }

        let new_name = req.name.unwrap_or(rule.name);
        let new_region = req.region.unwrap_or(rule.region);
        let new_tags = req.tags.or(rule.tags);
        let new_data_source = req.data_source.unwrap_or(rule.data_source);
        let new_datasource_id = req.datasource_id.or(rule.datasource_id);
        let new_sub_type = req.sub_type.unwrap_or(rule.sub_type);
        let new_threshold = req.threshold.unwrap_or(rule.threshold);
        let new_sr_version = req.starrocks_version.unwrap_or(rule.starrocks_version);
        let new_template_id = req.template_id.or(rule.template_id);
        let new_channel = req.channel.or(rule.channel);
        let new_ivr_template = req.ivr_template.or(rule.ivr_template);
        let new_ivr_params = req.ivr_params.or(rule.ivr_params);
        let new_ivr_secret = req.ivr_secret.or(rule.ivr_secret);
        let new_receivers = if let Some(r) = req.receivers {
            serde_json::to_value(r).unwrap()
        } else {
            serde_json::to_value(rule.receivers).unwrap()
        };
        let new_enabled = req.enabled.unwrap_or(rule.enabled);
        let new_auto_kill = req.auto_kill.unwrap_or(rule.auto_kill);
        let new_kill_threshold = req
            .auto_kill_threshold_minutes
            .or(rule.auto_kill_threshold_minutes);

        sqlx::query(
            "UPDATE warning_rule.alert_rules SET 
            name = ?, region = ?, tags = ?, data_source = ?, datasource_id = ?, sub_type = ?, 
            threshold = ?, starrocks_version = ?, template_id = ?, receivers = ?, enabled = ?,
            auto_kill = ?, auto_kill_threshold_minutes = ?, notify_interval_minutes = ?, channel = ?, ivr_template = ?, ivr_params = ?, ivr_secret = ?, channels = ?, updated_at = NOW()
            WHERE id = ?"
        )
        .bind(new_name)
        .bind(new_region)
        .bind(new_tags)
        .bind(new_data_source)
        .bind(new_datasource_id)
        .bind(new_sub_type)
        .bind(new_threshold)
        .bind(new_sr_version)
        .bind(new_template_id)
        .bind(new_receivers)
        .bind(new_enabled)
        .bind(new_auto_kill)
        .bind(new_kill_threshold)
        .bind(new_notify_interval)
        .bind(new_channel)
        .bind(new_ivr_template)
        .bind(new_ivr_params)
        .bind(new_ivr_secret)
        .bind(serde_json::to_value(req.channels.or(rule.channels).unwrap_or_default()).unwrap())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        // Immediate check for changed rule
        let _ = self.check_all_clusters().await;

        self.get_rule(id).await
    }

    pub async fn delete_rule(&self, id: i32) -> ApiResult<()> {
        sqlx::query("DELETE FROM warning_rule.alert_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;

        // Immediate check for deleted rule
        let _ = self.check_all_clusters().await;
        Ok(())
    }

    // --- History ---
    #[allow(clippy::collapsible_if)]
    pub async fn list_history(&self, query: HistoryQuery) -> ApiResult<AlertHistoryResponse> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);
        let offset = (page - 1) * page_size;

        let mut base_query = "FROM warning_rule.alert_history".to_string();
        let select_cols = "id, rule_id, query_id, CAST(start_time AS CHAR) as start_time, user, host, db, department, sql_text, violation_detail, status, alert_count, last_alert_time, cpu_time, mem_usage, exec_time, scan_rows, scan_bytes, connection_id, fe_ip, created_at, remark, repair_person, ivr_msg_id";
        let mut conditions = Vec::new();

        #[allow(clippy::collapsible_if)]
        if let Some(status) = &query.status {
            if !status.is_empty() {
                let statuses: Vec<String> = status
                    .split(',')
                    .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                    .collect();
                if !statuses.is_empty() {
                    conditions.push(format!("status IN ({})", statuses.join(",")));
                }
            }
        }

        #[allow(clippy::collapsible_if)]
        if let Some(cluster) = &query.cluster {
            if !cluster.is_empty() {
                conditions.push(format!("host = '{}'", cluster.replace('\'', "''")));
            }
        }

        if let Some(user) = &query.user {
            if !user.is_empty() {
                conditions.push(format!(
                    "(user LIKE '%{}%' OR query_id LIKE '%{}%')",
                    user.replace('\'', "''"),
                    user.replace('\'', "''")
                ));
            }
        }

        if let Some(qid) = &query.query_id {
            if !qid.is_empty() {
                conditions.push(format!("query_id LIKE '%{}%'", qid.replace('\'', "''")));
            }
        }

        if let Some(dept) = &query.department {
            if !dept.is_empty() {
                let depts: Vec<String> = dept
                    .split(',')
                    .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                    .collect();
                if !depts.is_empty() {
                    conditions.push(format!("department IN ({})", depts.join(",")));
                }
            }
        }

        if let Some(start) = &query.start_date {
            if !start.is_empty() {
                conditions.push(format!("created_at >= '{} 00:00:00'", start.replace('\'', "''")));
            }
        }

        if let Some(end) = &query.end_date {
            if !end.is_empty() {
                conditions.push(format!("created_at <= '{} 23:59:59'", end.replace('\'', "''")));
            }
        }

        if !conditions.is_empty() {
            base_query.push_str(" WHERE ");
            base_query.push_str(&conditions.join(" AND "));
        }

        let count_sql = format!("SELECT COUNT(*) {}", base_query);
        let total: i64 = sqlx::query_scalar(&count_sql)
            .fetch_one(&self.pool)
            .await
            .map_err(ApiError::database_error)?;

        // Handle sorting
        let mut order_clause = "ORDER BY created_at DESC".to_string();
        if let Some(field) = &query.sort_field {
            let allowed_fields = ["cpu_time", "mem_usage", "exec_time", "created_at", "scan_rows"];
            if allowed_fields.contains(&field.as_str()) {
                let order = query.sort_order.as_deref().unwrap_or("desc");
                let order_dir = if order == "asc" { "ASC" } else { "DESC" };
                order_clause = format!("ORDER BY {} {}, created_at DESC", field, order_dir);
            }
        }

        let list_sql = format!(
            "SELECT {} {} {} LIMIT {} OFFSET {}",
            select_cols, base_query, order_clause, page_size, offset
        );

        let items: Vec<AlertHistory> = sqlx::query_as(&list_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list alert history: {:?}", e);
                ApiError::database_error(e)
            })?;

        Ok(AlertHistoryResponse { items, total })
    }

    pub async fn get_history_clusters(&self) -> ApiResult<Vec<String>> {
        // Get unique hosts from history
        let mut result: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT host FROM warning_rule.alert_history WHERE host IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        result.sort();
        Ok(result)
    }

    pub async fn get_history_departments(&self) -> ApiResult<Vec<String>> {
        // Get unique departments from history
        let mut result: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT department FROM warning_rule.alert_history WHERE department IS NOT NULL AND department != ''"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        result.sort();
        Ok(result)
    }

    pub async fn update_remark(&self, id: i32, remark: String) -> ApiResult<()> {
        sqlx::query("UPDATE warning_rule.alert_history SET remark = ? WHERE id = ?")
            .bind(remark)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;
        Ok(())
    }

    pub async fn update_repair_person(&self, id: i32, repair_person: String) -> ApiResult<()> {
        sqlx::query("UPDATE warning_rule.alert_history SET repair_person = ? WHERE id = ?")
            .bind(repair_person)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(ApiError::database_error)?;
        Ok(())
    }

    pub async fn get_history_by_query_id(&self, query_id: &str) -> ApiResult<AlertHistory> {
        let cols = "id, rule_id, query_id, CAST(start_time AS CHAR) as start_time, user, host, db, department, sql_text, violation_detail, status, alert_count, last_alert_time, cpu_time, mem_usage, exec_time, scan_rows, scan_bytes, connection_id, fe_ip, created_at, remark, repair_person, ivr_msg_id";
        let res: Option<AlertHistory> = sqlx::query_as(&format!("SELECT {} FROM warning_rule.alert_history WHERE query_id = ? OR query_id LIKE ? ORDER BY created_at DESC LIMIT 1", cols))
            .bind(query_id)
            .bind(format!("{}%", query_id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query history by query_id {}: {:?}", query_id, e);
                ApiError::database_error(e)
            })?;

        res.ok_or_else(|| {
            tracing::warn!("History record not found for query_id: {}", query_id);
            ApiError::not_found(format!("History for query_id {} not found", query_id))
        })
    }

    pub async fn get_history_by_id(&self, id: i32) -> ApiResult<AlertHistory> {
        let cols = "id, rule_id, query_id, CAST(start_time AS CHAR) as start_time, user, host, db, department, sql_text, violation_detail, status, alert_count, last_alert_time, cpu_time, mem_usage, exec_time, scan_rows, scan_bytes, connection_id, fe_ip, created_at, remark, repair_person, ivr_msg_id";
        let res: Option<AlertHistory> = sqlx::query_as(&format!(
            "SELECT {} FROM warning_rule.alert_history WHERE id = ?",
            cols
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ApiError::database_error)?;

        res.ok_or_else(|| ApiError::not_found(format!("History record {} not found", id)))
    }

    pub async fn ensure_sql_text(&self, history: &mut AlertHistory) -> ApiResult<()> {
        #[allow(clippy::collapsible_if)]
        if let Some(sql) = &history.sql_text {
            if !sql.is_empty() && sql != "No SQL content" && !sql.contains("...") {
                return Ok(());
            }
        }

        // Attempt to fetch SQL
        // 1. Get Datasource through Rule
        let rule: Option<AlertRule> =
            sqlx::query_as("SELECT * FROM warning_rule.alert_rules WHERE id = ?")
                .bind(history.rule_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(ApiError::database_error)?;

        if let Some(r) = rule {
            let ds = if let Some(ds_id) = r.datasource_id {
                sqlx::query_as::<_, ResourceDataSource>(
                    "SELECT * FROM resource_data_sources WHERE id = ?",
                )
                .bind(ds_id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, ResourceDataSource>(
                    "SELECT * FROM resource_data_sources WHERE name = ?",
                )
                .bind(&r.data_source)
                .fetch_optional(&self.pool)
                .await?
            };

            if let Some(d) = ds {
                let cluster = self.ds_to_cluster(&d);
                #[allow(clippy::collapsible_if)]
                if let Some(sql) = self
                    .fetch_full_sql(&cluster, &history.query_id, history.fe_ip.as_deref())
                    .await
                {
                    if !sql.is_empty() {
                        // Update DB
                        sqlx::query(
                            "UPDATE warning_rule.alert_history SET sql_text = ? WHERE id = ?",
                        )
                        .bind(&sql)
                        .bind(history.id)
                        .execute(&self.pool)
                        .await
                        .map_err(ApiError::database_error)?;

                        history.sql_text = Some(sql);
                    }
                }
            }
        }

        Ok(())
    }
}

// Helpers
fn format_bytes(b: i64) -> String {
    const UNIT: i64 = 1024;
    // Handle 0 explicitly
    if b == 0 {
        return "0 B".to_string();
    }
    if b < UNIT {
        return format!("{} B", b);
    }
    let _div = UNIT;
    let _exp = (b as f64).ln() / (UNIT as f64).ln();
    // Simplified
    if b > 1024 * 1024 * 1024 {
        format!("{:.2} GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if b > 1024 * 1024 {
        format!("{:.2} MB", b as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} KB", b as f64 / 1024.0)
    }
}

// Improved time parsing that handles spaces better (e.g. "10 ms")
fn parse_time(s: &str) -> f64 {
    let s = s.replace(",", "").trim().to_lowercase();
    if s.is_empty() || s == "n/a" {
        return 0.0;
    }

    // Regex-like manual parsing
    let mut total_sec = 0.0;
    let mut current_num = String::new();

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '.' {
            current_num.push(c);
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else {
            // Start of unit
            let mut unit = String::new();
            while i < chars.len() && !chars[i].is_ascii_digit() && !chars[i].is_whitespace() {
                unit.push(chars[i]);
                i += 1;
            }

            let val = current_num.parse::<f64>().unwrap_or(0.0);
            current_num.clear();

            match unit.as_str() {
                "ms" => total_sec += val / 1000.0,
                "s" | "sec" => total_sec += val,
                "m" | "min" => total_sec += val * 60.0,
                "h" | "hour" => total_sec += val * 3600.0,
                _ => {}, // Unknown unit, ignore
            }
        }
    }

    // If number remains without unit (e.g. "1.5")
    if !current_num.is_empty() {
        total_sec += current_num.parse::<f64>().unwrap_or(0.0);
    }

    total_sec
}

// Improved size parsing
fn parse_size(s: &str) -> i64 {
    let s = s.replace(",", "").trim().to_uppercase();
    if s.is_empty() || s == "N/A" {
        return 0;
    }

    let mut val_str = String::new();
    let mut unit_str = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            val_str.push(c);
        } else if c.is_alphabetic() {
            unit_str.push(c);
        }
        // skip spaces
    }

    let val: f64 = val_str.parse().unwrap_or(0.0);

    let multiplier = match unit_str.as_str() {
        "K" | "KB" => 1024.0,
        "M" | "MB" => 1024.0 * 1024.0,
        "G" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "P" | "PB" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    (val * multiplier) as i64
}

impl AlertService {
    fn ds_to_cluster(&self, ds: &ResourceDataSource) -> Cluster {
        let (host, port) = self.parse_ds_url(&ds.url);
        Cluster {
            id: ds.id as i64,
            name: ds.name.clone(),
            description: None,
            fe_host: host,
            fe_http_port: 8030, // Default for now
            fe_query_port: port,
            username: ds.username.clone().unwrap_or_else(|| "root".to_string()),
            // Assuming password in DataSource is already encrypted or plain as needed by manager
            password_encrypted: ds.password.clone().unwrap_or_default(),
            enable_ssl: false,
            connection_timeout: ds.connection_timeout.unwrap_or(10),
            tags: None,
            catalog: "default_catalog".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            organization_id: None,
            deployment_mode: "shared_nothing".to_string(),
            fe_mapping: ds.fe_mapping.clone(),
        }
    }

    fn parse_ds_url(&self, url: &str) -> (String, i32) {
        let clean_url = url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("mysql://");

        // Remove trailing path/database if exists
        let host_port_part = clean_url.split('/').next().unwrap_or(clean_url);

        let parts: Vec<&str> = host_port_part.split(':').collect();
        if parts.len() == 2 {
            let host = parts[0].to_string();
            let port = parts[1].parse::<i32>().unwrap_or(9030);
            (host, port)
        } else {
            (host_port_part.to_string(), 9030)
        }
    }

    fn resolve_fe_endpoint(&self, cluster: &Cluster, ip: &str) -> (String, i32) {
        if let Some(mapping) = &cluster.fe_mapping {
            // 1. Try exact match with :9030 (standard SR port)
            let key_9030 = format!("{}:9030", ip);
            if let Some(mapped_val) = mapping.get(&key_9030).and_then(|v| v.as_str()) {
                let parts: Vec<&str> = mapped_val.split(':').collect();
                if parts.len() == 2 {
                    return (parts[0].to_string(), parts[1].parse::<i32>().unwrap_or(9030));
                }
                return (mapped_val.to_string(), 9030);
            }

            // 2. Try match on IP only
            if let Some(mapped_val) = mapping.get(ip).and_then(|v| v.as_str()) {
                let parts: Vec<&str> = mapped_val.split(':').collect();
                if parts.len() == 2 {
                    return (
                        parts[0].to_string(),
                        parts[1].parse::<i32>().unwrap_or(cluster.fe_query_port),
                    );
                }
                return (mapped_val.to_string(), cluster.fe_query_port);
            }
        }

        // Fallback: If no mapping found, use fe_ip from history and the cluster default query port
        (ip.to_string(), cluster.fe_query_port)
    }

    pub async fn get_sql_alert_summary(&self) -> ApiResult<serde_json::Value> {
        use sqlx::Row;
        let sql = r#"
            SELECT 
                CAST(COUNT(CASE WHEN created_at >= CURDATE() THEN 1 END) AS SIGNED) AS today_count,
                CAST(COUNT(CASE 
                    WHEN created_at >= CURDATE() - INTERVAL 1 DAY 
                    AND created_at <= NOW() - INTERVAL 1 DAY 
                    THEN 1 END) AS SIGNED) AS yesterday_same_period_count
            FROM warning_rule.alert_history 
            WHERE created_at >= CURDATE() - INTERVAL 1 DAY
        "#;

        let row = sqlx::query(sql).fetch_one(&self.pool).await?;

        let today_count: i64 = row.try_get(0).unwrap_or(0);
        let yesterday_count: i64 = row.try_get(1).unwrap_or(0);

        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM warning_rule.alert_history WHERE status = 'Alerting'",
        )
        .fetch_one(&self.pool)
        .await?;

        let percentage_change = if yesterday_count == 0 {
            if today_count == 0 { 0.0 } else { 100.0 }
        } else {
            ((today_count as f64 - yesterday_count as f64) / yesterday_count as f64 * 100.0 * 100.0)
                .round()
                / 100.0
        };

        tracing::info!(
            "SQL Alert Stats: today={}, yesterday={}, active={}, change={}",
            today_count,
            yesterday_count,
            active_count,
            percentage_change
        );

        Ok(json!({
            "todayCount": today_count,
            "activeCount": active_count,
            "yesterdayCount": yesterday_count,
            "percentageChange": percentage_change
        }))
    }

    pub async fn get_sql_alert_trend(&self, days: u32) -> ApiResult<serde_json::Value> {
        use sqlx::Row;
        let sql = format!(
            r#"
            SELECT 
                DATE_FORMAT(created_at, '%Y-%m-%d') as alert_date,
                CAST(COUNT(*) AS SIGNED) as alert_count
            FROM warning_rule.alert_history
            WHERE created_at >= DATE_SUB(CURDATE(), INTERVAL {} DAY)
            GROUP BY alert_date
            ORDER BY alert_date
        "#,
            days
        );

        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        let mut trend = Vec::new();
        for row in rows {
            let date: String = row
                .try_get("alert_date")
                .unwrap_or_else(|_| "Unknown".to_string());
            let count: i64 = row.try_get("alert_count").unwrap_or(0);
            trend.push(json!({
                "alert_date": date,
                "alert_count": count
            }));
        }
        tracing::info!("SQL Alert Trend: found {} days", trend.len());
        Ok(json!(trend))
    }
}

fn parse_count(s: &str) -> i64 {
    let s = s.replace(",", "");
    let numeric_part: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    match numeric_part.parse::<f64>() {
        Ok(val) => val as i64,
        Err(_) => 0,
    }
}
