// src/submit_batched_data.rs
use crate::extracts::metrics::SystemMetricsCollector;
use crate::{db::get_aurora_client, events::recorder::EventRecorder};
use sqlx::types::Json;

use anyhow::{Context, Result};

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

        // Insert each event into the database
        for event in data {
            let job_id = event.run_id.as_deref().unwrap_or("test-1234"); // Use a default if run_id is None
            let json_data = Json(serde_json::to_value(event)?); // Convert the event to JSON

            aurora_client
                .insert_row(job_id, json_data)
                .await
                .context("Failed to insert event into database")?;
        }

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
    use crate::exporters::FsExportHandler;
    use crate::extracts::metrics::SystemMetricsCollector;
    use anyhow::Result;
    use serde_json::Value;
    use sqlx::postgres::PgPool;
    use std::time::Duration;
    use sysinfo::System;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        // Load the configuration
        let config = ConfigManager::load_default_config();

        // Create an instance of AuroraClient
        let aurora_client = AuroraClient::new().await?;

        // Prepare test data
        let test_data = json!({
            "status": "completed",
            "execution_time": 45
        });

        let job_id = "job-12345";

        // Create a mock SystemMetricsCollector and EventRecorder
        let mut system = System::new();
        let mut logs = EventRecorder::default();
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

        // Verify the row was inserted into the database
        let result: (Json<Value>, String) =
            sqlx::query_as("SELECT data, job_id FROM batch_jobs_logs WHERE job_id = $1")
                .bind(job_id)
                .fetch_one(&aurora_client.pool) // Use the pool from the AuroraClient
                .await?;

        assert_eq!(result.0, Json(test_data)); // Compare with Json type
        assert_eq!(result.1, job_id);

        Ok(())
    }
}
