// src/tracer_client.rs
use crate::config_manager::ConfigFile;
use crate::event_recorder::EventRecorder;
use crate::metrics::SystemMetricsCollector;
use crate::process_watcher::ProcessWatcher;
use crate::submit_batched_data::submit_batched_data;
use anyhow::Result;
use std::time::{Duration, Instant};
use sysinfo::System;

pub struct TracerClient {
    system: System,
    last_sent: Option<Instant>,
    interval: Duration,
    pub logs: EventRecorder,
    process_watcher: ProcessWatcher,
    metrics_collector: SystemMetricsCollector,
    api_key: String,
    service_url: String,
}

impl TracerClient {
    pub fn new(config: ConfigFile) -> Result<TracerClient> {
        let service_url = config.service_url.clone();

        println!("Initializing TracerClient with API Key: {}", config.api_key);
        println!("Service URL: {}", service_url);

        Ok(TracerClient {
            // fixed values
            api_key: config.api_key,
            service_url,

            // updated values
            system: System::new_all(),
            last_sent: None,
            interval: Duration::from_millis(config.process_polling_interval_ms),
            // Sub mannagers
            logs: EventRecorder::new(),
            process_watcher: ProcessWatcher::new(config.targets),
            metrics_collector: SystemMetricsCollector::new(),
        })
    }

    pub fn reload_config_file(&mut self, config: &ConfigFile) {
        self.api_key.clone_from(&config.api_key);
        self.service_url.clone_from(&config.service_url);
        self.interval = Duration::from_millis(config.process_polling_interval_ms);
        self.process_watcher.reload_targets(config.targets.clone());
    }

    pub async fn submit_batched_data(&mut self) -> Result<()> {
        submit_batched_data(
            &self.api_key,
            &self.service_url,
            &mut self.system,
            &mut self.logs,
            &mut self.metrics_collector,
            &mut self.last_sent,
            self.interval,
        )
        .await
    }

    /// These functions require logs and the system
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

    pub fn refresh_sysinfo(&mut self) {
        self.system.refresh_all();
    }

    pub fn get_service_url(&self) -> &str {
        &self.service_url
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}
