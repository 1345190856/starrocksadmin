use sqlx::{MySqlPool, mysql::{MySqlPoolOptions, MySqlConnectOptions}};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    tracing::info!("Initializing database connection: {}", database_url);

    // Parse MySQL connection options
    let options = MySqlConnectOptions::from_str(database_url)
        .map_err(|e| sqlx::Error::Configuration(e.into()))?;

    // Create MySQL connection pool
    tracing::debug!("Creating database pool with max_connections=10, acquire_timeout=5s");
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| {
            tracing::error!("Database connection failed: {}", e);
            e
        })?;

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Migration failed: {}", e);
            e
        })?;
    tracing::info!("Database migrations completed successfully");
    tracing::info!("Database pool created successfully");

    Ok(pool)
}

#[allow(dead_code)]
fn find_migrations_dir() -> String {
    // Try different possible locations for migrations
    let possible_paths = [
        "./migrations",  // Production mode (when running from dist root)
        "../migrations", // When running from bin/
        "migrations",    // When running from project root
    ];

    for path in &possible_paths {
        if Path::new(path).exists() {
            tracing::debug!("Found migrations directory at: {}", path);
            return path.to_string();
        }
    }

    // Default fallback
    tracing::warn!("No migrations directory found, using default: ./migrations");
    "./migrations".to_string()
}
