// src/events/mod.rs
use crate::http_client::send_http_event;
use anyhow::{Context, Result};
use serde_json::json;
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

pub async fn send_log_event(
    service_url: &str,
    api_key: &str,
    status: EventStatus,
    message: &str,
) -> Result<()> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": status.to_string(),
        "event_type": "process_status"
    });

    send_http_event(service_url, api_key, &log_entry)
        .await
        .context("Failed to send HTTP event")
}

pub async fn send_message_event(service_url: &str, api_key: &str, message: String) -> Result<()> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "new_run",
        "event_type": "process_status"
    });

    send_http_event(service_url, api_key, &log_entry)
        .await
        .context("Failed to send HTTP event")
}

pub async fn send_alert_event(service_url: &str, api_key: &str, message: String) -> Result<()> {
    let alert_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": "alert",
        "event_type": "process_status"
    });

    send_http_event(service_url, api_key, &alert_entry)
        .await
        .context("Failed to send HTTP event")
}

pub async fn send_init_event(service_url: &str, api_key: &str) -> Result<()> {
    info!("Starting new pipeline...");

    let result = send_log_event(
        service_url,
        api_key,
        EventStatus::NewRun,
        "[CLI] Starting pipeline run",
    )
    .await
    .context("Failed to send HTTP event");

    info!("Started pipeline run successfully...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Error;

    #[tokio::test]
    async fn test_event_pipeline_run_start_new() -> Result<(), Error> {
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let result = send_init_event(&config.service_url.clone(), &config.api_key.clone()).await;

        //     //     assert!(result.is_ok(), "Expected success, but got an error");

        //     //     Ok(())
        //     // }
        //     #[tokio::test]
        //     async fn test_log_event() -> Result<(), Error> {
        //         let config = ConfigManager::load_config().context("Failed to load config")?;
        //         let message = "[shipping] Test log message from the test suite";

        //         let result = log_event(
        //             &config.service_url.clone(),
        //             &config.api_key.clone(),
        //             EventStatus::NewRun,
        //             message,
        //         )
        //         .await;

        //        let result = send_log_event(
        //            &config.service_url.clone(),
        //            &config.api_key.clone(),
        //            EventStatus::NewRun,
        //            message,
        //       )
        //        .await;
        //         assert!(result.is_ok(), "Expected success, but got an error");

        //         Ok(())
        //     }
        // }
        result
    }
}
