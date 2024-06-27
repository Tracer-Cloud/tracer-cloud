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
    api_key: String,
    system: System,
    service_url: String,
    last_sent: Instant,
    interval: Duration,
    pub logs: EventRecorder,
    process_watcher: ProcessWatcher,
    metrics_collector: SystemMetricsCollector,
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
            last_sent: Instant::now(),
            interval: Duration::from_millis(config.process_polling_interval_ms),
            // Sub mannagers
            logs: EventRecorder::new(),
            process_watcher: ProcessWatcher::new(config.targets),
            metrics_collector: SystemMetricsCollector::new(),
        })
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
}
