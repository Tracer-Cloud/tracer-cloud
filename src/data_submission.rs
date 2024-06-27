// src/data_submission.rs
use crate::event_recorder::EventRecorder;
use crate::http_client::HttpClient;
use crate::metrics::SystemMetricsCollector;

use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::Mutex;
use tracing::info;

pub async fn submit_batched_data(
    http_client: &HttpClient,
    system: &mut System,
    logs: &mut EventRecorder,
    metrics_collector: &mut SystemMetricsCollector,
    submitted_data: Arc<Mutex<Vec<String>>>,
    last_sent: &mut Option<Instant>,
    interval: Duration,
) -> Result<()> {
    if last_sent.is_none() || Instant::now() - last_sent.unwrap() >= interval {
        metrics_collector
            .collect_metrics(system, logs)
            .context("Failed to collect metrics")?;
        info!("Sending event to {} with API Key: {}", http_client.get_service_url(), http_client.get_api_key());

        let data = json!({ "logs": logs.get_events() });

        info!("Payload: {:#?}", data);

        let mut submitted_data = submitted_data.lock().await;
        submitted_data.push(data.to_string());

        *last_sent = Some(Instant::now());
        logs.clear();

        http_client
            .send_http_event(&data)
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
    use crate::http_client::HttpClient;
    use crate::metrics::SystemMetricsCollector;
    use anyhow::Result;
    use std::sync::Arc;
    use std::time::Duration;
    use sysinfo::System;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let service_url = config.service_url.clone();
        let api_key = config.api_key.clone();
        let http_client = HttpClient::new(service_url.clone(), api_key.clone());

        let mut system = System::new();
        let mut logs = EventRecorder::new();
        let mut metrics_collector = SystemMetricsCollector::new();
        let submitted_data = Arc::new(Mutex::new(Vec::new()));
        let mut last_sent = None;
        let interval = Duration::from_secs(60);

        // Record a test event
        logs.record_event(EventType::TestEvent, "Test event".to_string(), None);

        // Call the method to submit batched data
        submit_batched_data(
            &http_client,
            &mut system,
            &mut logs,
            &mut metrics_collector,
            submitted_data.clone(),
            &mut last_sent,
            interval,
        )
        .await?;

        // Retrieve the submitted data for verification
        let submitted_data = submitted_data.lock().await;

        // Assert that one batch of data was submitted and contains the test event
        assert_eq!(submitted_data.len(), 1);
        assert!(submitted_data[0].contains("Test event"));

        Ok(())
    }
}
