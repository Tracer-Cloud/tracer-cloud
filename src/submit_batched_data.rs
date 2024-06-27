// src/data_submission.rs
use crate::event_recorder::EventRecorder;
use crate::http_client::send_http_event;
use crate::metrics::SystemMetricsCollector;

use anyhow::{Context, Result};
use serde_json::json;
use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::info;

pub async fn submit_batched_data(
    api_key: &str,
    service_url: &str,
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

        let data = json!({ "logs": logs.get_events() });

        info!("Payload: {:#?}", data);

        *last_sent = Some(Instant::now());
        logs.clear();

        send_http_event(&service_url, &api_key, &data)
            .await
            .context("Failed to send HTTP event")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use crate::event_recorder::{EventRecorder, EventType};
    use crate::metrics::SystemMetricsCollector;
    use anyhow::Result;
    use std::time::Duration;
    use sysinfo::System;

    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let service_url = config.service_url.clone();
        let api_key = config.api_key.clone();

        let mut system = System::new();
        let mut logs = EventRecorder::new();
        let mut metrics_collector = SystemMetricsCollector::new();
        let mut last_sent = None;
        let interval = Duration::from_secs(60);

        // Record a test event
        logs.record_event(EventType::TestEvent, "Test event".to_string(), None);

        // Call the method to submit batched data
        submit_batched_data(
            &api_key,
            &service_url,
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
