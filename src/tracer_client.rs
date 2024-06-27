use crate::config_manager::ConfigFile;
use crate::event_recorder::{EventRecorder, EventType};
use crate::http_client::HttpClient;
use crate::metrics::SystemMetricsCollector;
use crate::process_watcher::ProcessWatcher;
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::Mutex;

pub struct TracerClient {
    http_client: HttpClient,
    api_key: String,
    system: System,
    service_url: String,
    last_sent: Instant,
    interval: Duration,
    logs: EventRecorder,
    process_watcher: ProcessWatcher,
    metrics_collector: SystemMetricsCollector,
    submitted_data: Arc<Mutex<Vec<String>>>,
}

impl TracerClient {
    pub fn new(config: ConfigFile) -> Result<TracerClient> {
        let service_url = config.service_url.clone();

        println!("Initializing TracerClient with API Key: {}", config.api_key);
        println!("Service URL: {}", service_url);

        Ok(TracerClient {
            http_client: HttpClient::new(service_url.clone(), config.api_key.clone()),
            api_key: config.api_key,
            system: System::new_all(),
            last_sent: Instant::now(),
            interval: Duration::from_millis(config.process_polling_interval_ms),
            logs: EventRecorder::new(),
            service_url,
            process_watcher: ProcessWatcher::new(config.targets),
            metrics_collector: SystemMetricsCollector::new(),
            submitted_data: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn submit_batched_data(&mut self) -> Result<()> {
        if Instant::now() - self.last_sent >= self.interval {
            self.metrics_collector
                .collect_metrics(&mut self.system, &mut self.logs)?;
            println!(
                "Sending event to {} with API Key: {}",
                self.service_url, self.api_key
            );

            let data = json!({ "logs": self.logs.get_events() });

            println!("{:#?}", data); // Log to file located at `/tmp/tracerd.out`

            // Store the submitted data for testing purposes
            let mut submitted_data = self.submitted_data.lock().await;
            submitted_data.push(data.to_string());

            self.last_sent = Instant::now();
            self.logs.clear();

            self.http_client.send_http_event(&data).await
        } else {
            Ok(())
        }
    }

    pub async fn poll_processes(&mut self) -> Result<()> {
        self.process_watcher
            .poll_processes(&mut self.system, &mut self.logs)?;
        Ok(())
    }

    pub async fn remove_completed_processes(&mut self) -> Result<()> {
        self.process_watcher
            .remove_completed_processes(&mut self.system, &mut self.logs)?;
        Ok(())
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    // New methods for testing
    #[allow(dead_code)]
    pub async fn get_submitted_data(&self) -> Vec<String> {
        self.submitted_data.lock().await.clone()
    }

    #[allow(dead_code)]
    pub fn get_processes_count(&self) -> usize {
        self.process_watcher.get_monitored_processes_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::{Context, Result};

    // Change in the test function to correctly handle the asynchronous config loading
    #[tokio::test]
    async fn test_submit_batched_data() -> Result<()> {
        // Correctly await the asynchronous function
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let mut tracer_client =
            TracerClient::new(config.clone()).context("Failed to create TracerClient")?;

        // Record a test event
        tracer_client
            .logs
            .record_event(EventType::TestEvent, "Test event".to_string(), None);

        // Call the method to submit batched data
        tracer_client.submit_batched_data().await?;

        // Retrieve the submitted data for verification
        let submitted_data = tracer_client.get_submitted_data().await;

        // Assert that one batch of data was submitted and contains the test event
        assert_eq!(submitted_data.len(), 1);
        assert!(submitted_data[0].contains("Test event"));

        Ok(())
    }
}
