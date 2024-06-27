// src/events/mod.rs
use crate::http_client::HttpClient;
use anyhow::{Context, Result};
use serde_json::json;
use tracing::{info, instrument};

#[derive(Debug)]
pub enum EventStatus {
    #[allow(dead_code)]
    NewRun,
}

impl ToString for EventStatus {
    fn to_string(&self) -> String {
        match self {
            EventStatus::NewRun => "new_run".to_string(),
        }
    }
}

#[instrument(skip(http_client))]
pub async fn event_pipeline_run_start_new(http_client: &HttpClient) -> Result<()> {
    info!("Starting new pipeline...");

    log_event(
        http_client,
        EventStatus::NewRun,
        "[CLI] Starting pipeline run",
    )
    .await
    .context("Failed to log event")?;

    info!("Started pipeline run successfully...");
    Ok(())
}

#[instrument(skip(http_client))]
async fn log_event(http_client: &HttpClient, status: EventStatus, message: &str) -> Result<()> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline",
        "process_status": status.to_string(),
        "event_type": "process_status"
    });

    http_client
        .send_http_event(&log_entry)
        .await
        .context("Failed to send HTTP event")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Error;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn initialize() {
        INIT.call_once(|| {
            let _ = env_logger::builder().is_test(true).try_init();
        });
    }

    async fn initialize_http_client() -> Result<HttpClient> {
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let http_client = HttpClient::new(config.service_url, config.api_key);
        Ok(http_client)
    }

    async fn create_test_http_client() -> Result<HttpClient> {
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let http_client = HttpClient::new(config.service_url, config.api_key);
        Ok(http_client)
    }

    #[tokio::test]
    async fn test_event_pipeline_run_start_new() -> Result<(), Error> {
        initialize();

        let http_client = create_test_http_client().await?;
        let result = event_pipeline_run_start_new(&http_client).await;

        assert!(result.is_ok(), "Expected success, but got an error");

        Ok(())
    }

    #[tokio::test]
    async fn test_log_event() -> Result<(), Error> {
        initialize();

        let http_client = create_test_http_client().await?;
        let message = "[shipping] Test log message from the test suite";

        let result = log_event(&http_client, EventStatus::NewRun, message).await;

        assert!(result.is_ok(), "Expected success, but got an error");

        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_http_client() -> Result<(), Error> {
        initialize();

        let result = initialize_http_client().await;

        assert!(result.is_ok(), "Expected success, but got an error");
        Ok(())
    }
}
