use crate::models::data_sync::{Secret, SyncSubmitRequest};
use sqlx::MySqlPool;

pub struct DataSyncService {
    db: MySqlPool,
}

impl DataSyncService {
    pub fn new(db: MySqlPool) -> Self {
        Self { db }
    }

    pub async fn get_secret(
        &self,
        host: &str,
        port: &str,
        region: &str,
    ) -> anyhow::Result<Option<Secret>> {
        let port_int: i32 = port.parse().unwrap_or(3306);
        let secret = sqlx::query_as::<_, Secret>(
            "SELECT * FROM data_sync.secret WHERE (host = ? OR ip = ?) AND port = ? AND region = ?",
        )
        .bind(host)
        .bind(host)
        .bind(port_int)
        .bind(region)
        .fetch_optional(&self.db)
        .await?;
        Ok(secret)
    }

    pub async fn submit_ticket(&self, req: SyncSubmitRequest, creator: &str) -> anyhow::Result<()> {
        let tables_json = serde_json::to_string(&req.selected_tables)?;

        let source_secret = self
            .get_secret(&req.source_ip, &req.source_port, &req.country)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Source host {}:{} credentials not found for region {}",
                    req.source_ip,
                    req.source_port,
                    req.country
                )
            })?;

        let dest_secret = self
            .get_secret(&req.dest_ip, &req.dest_port, &req.country)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Destination host {}:{} credentials not found for region {}",
                    req.dest_ip,
                    req.dest_port,
                    req.country
                )
            })?;

        let processor = if req.country == "墨西哥" || req.country == "菲律宾" {
            "侯世涛"
        } else {
            "刘壮"
        };

        sqlx::query(
            "INSERT INTO data_sync.sync_list (
                country, source_ip, source_port, source_username, source_password,
                dest_ip, dest_port, dest_username, dest_password,
                selected_tables, approval_status, creator, processor, remark
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(req.country)
        .bind(req.source_ip)
        .bind(req.source_port)
        .bind(source_secret.username)
        .bind(source_secret.password)
        .bind(req.dest_ip)
        .bind(req.dest_port)
        .bind(dest_secret.username)
        .bind(dest_secret.password)
        .bind(tables_json)
        .bind("待审批")
        .bind(creator)
        .bind(processor)
        .bind(req.remark)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn list_tickets(&self) -> anyhow::Result<Vec<crate::models::data_sync::SyncList>> {
        let list = sqlx::query_as::<_, crate::models::data_sync::SyncList>(
            "SELECT * FROM data_sync.sync_list ORDER BY created_at DESC",
        )
        .fetch_all(&self.db)
        .await?;
        Ok(list)
    }

    pub async fn update_processor(&self, id: i32, processor: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE data_sync.sync_list SET processor = ? WHERE id = ?")
            .bind(processor)
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: i32, status: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE data_sync.sync_list SET approval_status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn approve_ticket(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query("UPDATE data_sync.sync_list SET approval_status = '已完成', finished_at = NOW() WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}
