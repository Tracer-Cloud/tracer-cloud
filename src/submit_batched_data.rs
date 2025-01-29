// src/submit_batched_data.rs
use crate::metrics::SystemMetricsCollector;
use crate::{event_recorder::EventRecorder, exporters::ParquetExport};

use anyhow::{Context, Result};

use std::time::{Duration, Instant};
use sysinfo::System;

pub async fn submit_batched_data(
    run_name: &str,
    exporter: &mut impl ParquetExport,
    system: &mut System,
    logs: &mut EventRecorder, // Todo and change: there should be a distinction between logs array and event recorder. The logs appears as vector while it isn't
    metrics_collector: &mut SystemMetricsCollector,
    last_sent: &mut Option<Instant>,
    interval: Duration,
) -> Result<()> {
    if last_sent.is_none() || Instant::now() - last_sent.unwrap() >= interval {
        metrics_collector
            .collect_metrics(system, logs)
            .context("Failed to collect metrics")?;

        let data = logs.get_events();
        match exporter.output(data, run_name).await {
            Ok(_path) => {
                // upload to s3
                println!("Successfully outputed, uploading to s3");
            }
            Err(err) => println!("error outputing parquet file: {err}"),
        };
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
    use crate::event_recorder::{EventRecorder, EventType};
    use crate::exporters::FsExportHandler;
    use crate::metrics::SystemMetricsCollector;
    use anyhow::Result;
    use std::time::Duration;
    use sysinfo::System;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        let mut system = System::new();
        let mut logs = EventRecorder::default();
        let mut metrics_collector = SystemMetricsCollector::new();
        let mut last_sent = None;
        let interval = Duration::from_secs(60);
        let temp_dir = TempDir::new("tracer-client-events").expect("failed to create tempdir");

        let base_dir = temp_dir.path().join("./exports");
        let mut exporter = FsExportHandler::new(base_dir, None);

        // Record a test event
        logs.record_event(
            EventType::TestEvent,
            "[submit_batched_data.rs] Test event".to_string(),
            None,
            None,
        );

        // Call the method to submit batched data
        submit_batched_data(
            "test_run",
            &mut exporter,
            &mut system,
            &mut logs,
            &mut metrics_collector,
            &mut last_sent,
            interval,
        )
        .await?;

        Ok(())
    }
}
