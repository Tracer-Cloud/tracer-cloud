// src/events/mod.rs
use crate::{
    debug_log::Logger,
    http_client::send_http_event,
    metrics::SystemMetricsCollector,
    types::event::{
        attributes::system_metrics::SystemProperties, aws_metadata::AwsInstanceMetaData,
    },
};
mod run_details;
use anyhow::{Context, Result};
use chrono::Utc;
use run_details::{generate_run_id, generate_run_name};
use serde_json::json;
use sysinfo::System;
use tracing::info;

#[derive(Debug)]
pub enum EventStatus {
    #[allow(dead_code)]
    NewRun,
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EventStatus::NewRun => "new_run".to_string(),
            }
        )
    }
}

pub async fn send_log_event(service_url: &str, api_key: &str, message: String) -> Result<String> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "run_status_message",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &log_entry)
        .await
        .context("Failed to send HTTP event")
}

pub async fn send_alert_event(service_url: &str, api_key: &str, message: String) -> Result<String> {
    let alert_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "alert",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &alert_entry)
        .await
        .context("Failed to send HTTP event")
}

pub struct RunEventOut {
    pub run_name: String,
    pub run_id: String,
    pub system_properties: SystemProperties,
}

async fn get_aws_instance_metadata() -> Option<AwsInstanceMetaData> {
    let client = ec2_instance_metadata::InstanceMetadataClient::new();
    match client.get() {
        Ok(metadata) => Some(metadata.into()),
        Err(err) => {
            println!("error getting metadata: {err}");
            None
        }
    }
}

async fn gather_system_properties(system: &System) -> SystemProperties {
    let aws_metadata = get_aws_instance_metadata().await;
    let is_aws_instance = aws_metadata.is_some();

    let system_disk_io = SystemMetricsCollector::gather_disk_data();

    SystemProperties {
        os: System::name(),
        os_version: System::os_version(),
        kernel_version: System::kernel_version(),
        arch: System::cpu_arch(),
        num_cpus: system.cpus().len(),
        hostname: System::host_name(),
        total_memory: system.total_memory(),
        total_swap: system.total_swap(),
        uptime: System::uptime(),
        aws_metadata,
        is_aws_instance,
        system_disk_io,
    }
}

// NOTE: moved pipeline_name to tracer client
pub async fn send_start_run_event(system: &System, pipeline_name: &str) -> Result<RunEventOut> {
    info!("Starting new pipeline...");

    let logger = Logger::new();

    let system_properties = gather_system_properties(system).await;

    let run_name = generate_run_name();

    let run_id = generate_run_id();

    logger
        .log(
            format!("New pipeline {} run initiated", &pipeline_name).as_str(),
            None,
        )
        .await;

    logger
        .log(
            format!(
                "Run name: {}, run id: {}, service name: {}",
                run_name, run_id, pipeline_name
            )
            .as_str(),
            None,
        )
        .await;

    info!("Started pipeline run successfully...");

    Ok(RunEventOut {
        run_name: run_name.clone(),
        run_id: run_id.clone(),
        system_properties,
    })
}

// TODO: remove
pub async fn send_end_run_event(service_url: &str, api_key: &str) -> Result<String> {
    info!("Finishing pipeline run...");

    let end_entry = json!({
        "message": "[CLI] Finishing pipeline run",
        "process_type": "pipeline",
        "process_status": "finished_run",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    let result = send_http_event(service_url, api_key, &end_entry).await;

    info!("Ended pipeline run successfully...");
    result
}

pub async fn send_daemon_start_event(service_url: &str, api_key: &str) -> Result<String> {
    let daemon_start_entry: serde_json::Value = json!({
        "message": "[CLI] Starting daemon",
        "process_type": "pipeline",
        "process_status": "daemon_start",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &daemon_start_entry).await
}

// TODO: Should tag updates be parts of events?
pub async fn send_update_tags_event(
    service_url: &str,
    api_key: &str,
    tags: Vec<String>,
) -> Result<String> {
    let tags_entry = json!({
        "tags": tags,
        "message": "[CLI] Updating tags",
        "process_type": "pipeline",
        "process_status": "tag_update",
        "event_type": "process_status",
        "timestamp": Utc::now().timestamp_millis() as f64 / 1000.,
    });

    send_http_event(service_url, api_key, &tags_entry).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Error;

    #[tokio::test]
    async fn test_event_log() -> Result<(), Error> {
        let config = ConfigManager::load_default_config();
        send_log_event(
            &config.service_url.clone(),
            &config.api_key.clone(),
            "Test".to_string(),
        )
        .await?;

        Ok(())
    }
}
