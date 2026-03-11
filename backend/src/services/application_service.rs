use crate::models::{Application, CreateApplicationRequest, UpdateApplicationRequest};
use crate::utils::ApiResult;
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct ApplicationService {
    pool: MySqlPool,
}

impl ApplicationService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_applications(&self) -> ApiResult<Vec<Application>> {
        let apps = sqlx::query_as::<_, Application>("SELECT * FROM applications ORDER BY region, name")
            .fetch_all(&self.pool)
            .await?;
        Ok(apps)
    }

    pub async fn create_application(&self, req: CreateApplicationRequest) -> ApiResult<Application> {
        let result = sqlx::query(
            "INSERT INTO applications (name, type, address, region) VALUES (?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.r#type)
        .bind(&req.address)
        .bind(&req.region)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_id();
        let app = sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(app)
    }

    pub async fn update_application(&self, id: i32, req: UpdateApplicationRequest) -> ApiResult<Application> {
        let mut tx = self.pool.begin().await?;

        if let Some(name) = &req.name {
            sqlx::query("UPDATE applications SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(r#type) = &req.r#type {
            sqlx::query("UPDATE applications SET type = ? WHERE id = ?")
                .bind(r#type)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(address) = &req.address {
            sqlx::query("UPDATE applications SET address = ? WHERE id = ?")
                .bind(address)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(region) = &req.region {
            sqlx::query("UPDATE applications SET region = ? WHERE id = ?")
                .bind(region)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        let app = sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(app)
    }

    pub async fn delete_application(&self, id: i32) -> ApiResult<()> {
        sqlx::query("DELETE FROM applications WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
