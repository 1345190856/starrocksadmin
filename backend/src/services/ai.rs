use sqlx::MySqlPool;
use crate::models::ai::{AiSetting, CreateAiSettingRequest, UpdateAiSettingRequest};
use anyhow::Result;
use tracing::{debug, info};

pub struct AiService {
    pool: MySqlPool,
}

impl AiService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_settings(&self, username: &str, is_super_admin: bool) -> Result<Vec<AiSetting>> {
        info!("Listing AI settings for user: {}, is_admin: {}", username, is_super_admin);
        let settings = if is_super_admin || username == "admin" {
            debug!("Admin user detected, returning all settings");
            sqlx::query_as::<_, AiSetting>(
                "SELECT * FROM ai_settings ORDER BY id ASC"
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            debug!("Regular user detected, filtering by published or creator");
            sqlx::query_as::<_, AiSetting>(
                "SELECT * FROM ai_settings WHERE is_published = 1 OR creator = ? ORDER BY id ASC"
            )
            .bind(username)
            .fetch_all(&self.pool)
            .await?
        };
        debug!("Found {} AI settings", settings.len());
        Ok(settings)
    }

    pub async fn get_setting(&self, id: i32) -> Result<AiSetting> {
        let setting = sqlx::query_as::<_, AiSetting>(
            "SELECT * FROM ai_settings WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(setting)
    }

    pub async fn create_setting(&self, req: CreateAiSettingRequest, creator: &str) -> Result<AiSetting> {
        let is_published = if req.is_published.unwrap_or(false) { 1i8 } else { 0i8 };
        let res = sqlx::query(
            "INSERT INTO ai_settings (name, url, body, category, is_published, creator) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.url)
        .bind(&req.body)
        .bind(&req.category)
        .bind(is_published)
        .bind(creator)
        .execute(&self.pool)
        .await?;

        let id = res.last_insert_id();
        let setting = sqlx::query_as::<_, AiSetting>(
            "SELECT * FROM ai_settings WHERE id = ?"
        )
        .bind(id as i32)
        .fetch_one(&self.pool)
        .await?;

        Ok(setting)
    }

    pub async fn update_setting(&self, id: i32, req: UpdateAiSettingRequest, username: &str, is_super_admin: bool) -> Result<AiSetting> {
        // Check ownership
        let current = self.get_setting(id).await?;
        if !is_super_admin && current.creator.as_deref() != Some(username) && username != "admin" {
            return Err(anyhow::anyhow!("Permission denied: not the owner"));
        }

        let is_published = if req.is_published { 1i8 } else { 0i8 };

        sqlx::query(
            "UPDATE ai_settings SET name = ?, url = ?, body = ?, category = ?, is_published = ? WHERE id = ?"
        )
        .bind(&req.name)
        .bind(&req.url)
        .bind(&req.body)
        .bind(&req.category)
        .bind(is_published)
        .bind(id)
        .execute(&self.pool)
        .await?;

        let setting = sqlx::query_as::<_, AiSetting>(
            "SELECT * FROM ai_settings WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(setting)
    }

    pub async fn delete_setting(&self, id: i32, username: &str, is_super_admin: bool) -> Result<()> {
        // Check ownership
        let current = self.get_setting(id).await?;
        if !is_super_admin && current.creator.as_deref() != Some(username) && username != "admin" {
            return Err(anyhow::anyhow!("Permission denied: not the owner"));
        }

        sqlx::query("DELETE FROM ai_settings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
