use crate::extracts::metrics::SystemMetricsCollector;
use crate::{db::get_aurora_client, events::recorder::EventRecorder};
use anyhow::{Context, Result};
use sqlx::types::Json;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use sysinfo::System;

pub async fn submit_batched_data(
    _run_name: &str,
    system: &mut System,
    logs: &mut EventRecorder,
    metrics_collector: &mut SystemMetricsCollector,
    last_sent: &mut Option<Instant>,
    interval: Duration,
) -> Result<()> {
    if last_sent.is_none() || Instant::now() - last_sent.unwrap() >= interval {
        metrics_collector
            .collect_metrics(system, logs)
            .context("Failed to collect metrics")?;

        let data = logs.get_events();

        // Get the AuroraClient instance from the singleton
        let aurora_client = get_aurora_client().await;
        let pool: &PgPool = aurora_client.get_pool(); // Use the get_pool method

        // Start a transaction
        let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

        // Insert each event into the database
        for event in data {
            let job_id = event.run_id.as_deref().unwrap_or("job-1234"); // Use a default if run_id is None
            let json_data = Json(serde_json::to_value(event)?); // Convert the event to JSON

            // Pass the transaction directly (not as a mutable reference)
            sqlx::query("INSERT INTO batch_jobs_logs (data, job_id) VALUES ($1, $2)")
                .bind(json_data)
                .bind(job_id)
                .execute(&mut *transaction) // Use the transaction directly
                .await
                .context("Failed to insert event into database")?;
        }

        // Commit the transaction
        transaction
            .commit()
            .await
            .context("Failed to commit transaction")?;

        *last_sent = Some(Instant::now());
        logs.clear();

        Ok(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use crate::db::aurora_client::AuroraClient;
    use crate::events::recorder::{EventRecorder, EventType};
    use crate::extracts::metrics::SystemMetricsCollector;
    use anyhow::Result;
    use serde_json::json;
    use serde_json::Value;
    use std::time::Duration;
    use sysinfo::System;

    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        // Load the configuration
        let _config = ConfigManager::load_default_config();

        // Create an instance of AuroraClient
        let aurora_client = AuroraClient::new().await?;

        // Prepare test data
        let _test_data = json!({
            "status": "completed",
            "execution_time": 45
        });

        let job_id = "job-1234";

        // Create a mock SystemMetricsCollector and EventRecorder
        let mut system = System::new();
        let mut logs = EventRecorder::new(
            Some("test_name".to_string()),
            Some(job_id.to_string()),
            Some(job_id.to_string()),
        );

        let mut metrics_collector = SystemMetricsCollector::new();
        let mut last_sent = None;
        let interval = Duration::from_secs(60);

        // Record a test event
        logs.record_event(
            EventType::TestEvent,
            format!("[submit_batched_data.rs] Test event for job {}", job_id),
            None,
            None,
        );

        // Call the method to submit batched data
        submit_batched_data(
            "test_run",
            &mut system,
            &mut logs,
            &mut metrics_collector,
            &mut last_sent,
            interval,
        )
        .await?;

        // Prepare the SQL query
        let query = "SELECT data, job_id FROM batch_jobs_logs WHERE job_id = $1";

        // Verify the row was inserted into the database
        let result: (Json<Value>, String) = sqlx::query_as(query)
            .bind(job_id) // Use the job_id for the query
            .fetch_one(aurora_client.get_pool()) // Use the pool from the AuroraClient
            .await?;

        // Check that the inserted data matches the expected data
        assert_eq!(result.1, job_id); // Compare with the unique job ID

        Ok(())
    }
}
