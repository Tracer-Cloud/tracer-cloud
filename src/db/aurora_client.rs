use anyhow::{Context, Result};
use log::info;
use serde_json::Value;
use sqlx::pool::PoolOptions;
use sqlx::types::Json;
use sqlx::PgPool;

pub struct AuroraClient {
    pool: PgPool,
}

impl AuroraClient {
    pub async fn new() -> Result<Self, anyhow::Error> {
        // Hardcoded database connection string (to change)
        let db_url = "postgres://postgres:tracer-test@tracer-database.cdgizpzxtdp6.us-east-1.rds.amazonaws.com:5432/postgres";

        // Use PgPoolOptions to set max_size
        let pool = PoolOptions::new()
            .max_connections(100)
            .connect(db_url)
            .await?;

        info!("Successfully created connection pool");

        Ok(AuroraClient { pool })
    }

    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert_row(&self, job_id: &str, data: Json<Value>) -> Result<()> {
        let query = "INSERT INTO batch_jobs_logs (data, job_id) VALUES ($1, $2)";

        info!("Inserting row with job_id: {}", job_id);

        sqlx::query(query)
            .bind(data)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .context("Failed to insert row")?;

        info!("Successfully inserted row with job_id: {}", job_id);

        Ok(())
    }
}
