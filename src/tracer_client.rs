// src/tracer_client.rs
use crate::config_manager::ConfigFile;
use crate::data_submission::submit_batched_data;
use crate::event_recorder::EventRecorder;
use crate::http_client::HttpClient;
use crate::metrics::SystemMetricsCollector;
use crate::process_watcher::ProcessWatcher;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::Mutex;

pub struct TracerClient {
    pub http_client: HttpClient,
    api_key: String,
    system: System,
    service_url: String,
    last_sent: Instant,
    interval: Duration,
    pub logs: EventRecorder,
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
        submit_batched_data(
            &self.http_client,
            &self.api_key,
            &self.service_url,
            &mut self.system,
            &mut self.logs,
            &mut self.metrics_collector,
            self.submitted_data.clone(),
            &mut self.last_sent,
            self.interval,
        )
        .await
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
