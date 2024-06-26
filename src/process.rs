/// src/process.rs
use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::{time::Duration, time::Instant};
use sysinfo::{Disks, System};

use crate::config_manager::ConfigFile;
use crate::event_recorder::{EventRecorder, EventType};
use crate::http_client::HttpClient;
use crate::process_watcher::ProcessWatcher;

pub struct TracerClient {
    http_client: HttpClient,
    api_key: String,
    system: System,
    service_url: String,
    last_sent: Instant,
    interval: Duration,
    logs: EventRecorder,
    process_watcher: ProcessWatcher,
}

impl TracerClient {
    pub fn from_config(config: ConfigFile) -> Result<TracerClient> {
        let service_url = "https://app.tracer.bio/api/data-collector-api".to_string();

        println!("Initializing TracerClient with API Key: {}", config.api_key);
        println!("Service URL: {}", service_url);

        Ok(TracerClient {
            http_client: HttpClient::new(service_url.clone(), config.api_key.clone()),
            api_key: config.api_key,
            system: System::new_all(),
            last_sent: Instant::now(),
            interval: Duration::from_millis(config.polling_interval_ms),
            logs: EventRecorder::new(),
            service_url,
            process_watcher: ProcessWatcher::new(config.targets),
        })
    }

    pub async fn send_event(client: &mut TracerClient) -> Result<()> {
        if Instant::now() - client.last_sent >= client.interval {
            TracerClient::send_global_stat(&mut client.system, &mut client.logs)?;
            println!(
                "Sending event to {} with API Key: {}",
                client.service_url, client.api_key
            );

            let data = json!({ "logs": client.logs.get_events() });

            println!("{:#?}", data); // Log to file located at `/tmp/tracerd.out`

            client.last_sent = Instant::now();
            client.logs.clear();

            client.http_client.send_http_event(&data).await
        } else {
            Ok(())
        }
    }

    pub async fn poll_processes(tracer_client: &mut TracerClient) -> Result<()> {
        tracer_client
            .process_watcher
            .poll_processes(&mut tracer_client.system, &mut tracer_client.logs)?;
        Ok(())
    }

    pub async fn remove_completed_processes(tracer_client: &mut TracerClient) -> Result<()> {
        tracer_client
            .process_watcher
            .remove_completed_processes(&mut tracer_client.system, &mut tracer_client.logs)?;
        Ok(())
    }

    // Sends current load of a system to the server
    fn send_global_stat(system: &mut System, logs: &mut EventRecorder) -> Result<()> {
        let used_memory = system.used_memory();
        let total_memory = system.total_memory();
        let memory_utilization = (used_memory as f64 / total_memory as f64) * 100.0;

        let cpu_usage = system.global_cpu_info().cpu_usage();

        // please fix:

        let disks = Disks::new_with_refreshed_list();

        let mut d_stats = vec![];

        for d in disks.iter() {
            let Some(d_name) = d.name().to_str() else {
                continue;
            };

            let total_space = d.total_space();
            let available_space = d.available_space();
            let used_space = total_space - available_space;
            let disk_utilization = (used_space as f64 / total_space as f64) * 100.0;

            let disk_data = json!({
                d_name: {
                  "disk_total_space": total_space,
                  "disk_used_space": used_space,
                  "disk_available_space": available_space,
                  "disk_utilization": disk_utilization,
                },
            });

            d_stats.push(disk_data);
        }

        let attributes = json!({
            "events_name": "global_system_metrics",
            "total_memory": total_memory,
            "used_memory": used_memory,
            "available_memory": system.available_memory(),
            "memory_utilization": memory_utilization,
            "cpu_usage_percentage": cpu_usage,
            "disk_data": d_stats,
        });

        logs.record(
            EventType::MetricEvent,
            format!("[{}] System's resources metric", Utc::now()),
            Some(attributes),
        );

        Ok(())
    }

    pub fn refresh(tracer_client: &mut TracerClient) {
        tracer_client.system.refresh_all();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_conf() -> ConfigFile {
        toml::from_str(
            &std::fs::read_to_string(
                std::env::var("TRACER_CONFIG").unwrap_or("tracer.toml".to_string()),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn from_config() {
        let tr = TracerClient::from_config(create_conf());
        assert!(tr.is_ok())
    }

    #[tokio::test]
    async fn tool_exec() {
        let mut tr = TracerClient::from_config(create_conf()).unwrap();
        tr.process_watcher = ProcessWatcher::new(vec!["sleep".to_string()]);

        let mut cmd = std::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .unwrap();

        while tr.process_watcher.get_seen().is_empty() {
            TracerClient::refresh(&mut tr);
            TracerClient::poll_processes(&mut tr).await.unwrap();
        }

        cmd.wait().unwrap();

        assert!(!tr.process_watcher.get_seen().is_empty())
    }
}
