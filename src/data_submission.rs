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
    api_key: &str,
    service_url: &str,
    system: &mut System,
    logs: &mut EventRecorder,
    metrics_collector: &mut SystemMetricsCollector,
    submitted_data: Arc<Mutex<Vec<String>>>,
    last_sent: &mut Instant,
    interval: Duration,
) -> Result<()> {
    if Instant::now() - *last_sent >= interval {
        metrics_collector
            .collect_metrics(system, logs)
            .context("Failed to collect metrics")?;
        info!("Sending event to {} with API Key: {}", service_url, api_key);

        let data = json!({ "logs": logs.get_events() });

        info!("Payload: {:#?}", data);

        let mut submitted_data = submitted_data.lock().await;
        submitted_data.push(data.to_string());

        *last_sent = Instant::now();
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
    use std::time::{Duration, Instant};
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
        let mut last_sent = Instant::now() - Duration::from_secs(3600); // Set to a past time
        let interval = Duration::from_secs(60);

        // Record a test event
        logs.record_event(EventType::TestEvent, "Test event".to_string(), None);

        // Call the method to submit batched data
        submit_batched_data(
            &http_client,
            &api_key,
            &service_url,
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
