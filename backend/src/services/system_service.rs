use crate::models::system::{SystemConfig, UpdateSystemConfigRequest};
use crate::utils::ApiResult;
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct SystemService {
    pool: MySqlPool,
}

impl SystemService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn get_config(&self, key: &str) -> ApiResult<Option<SystemConfig>> {
        let config =
            sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_config WHERE config_key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(config)
    }

    pub async fn update_config(&self, key: &str, req: UpdateSystemConfigRequest) -> ApiResult<()> {
        sqlx::query("INSERT INTO system_config (config_key, config_value) VALUES (?, ?) ON DUPLICATE KEY UPDATE config_value = ?")
            .bind(key)
            .bind(&req.config_value)
            .bind(&req.config_value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
