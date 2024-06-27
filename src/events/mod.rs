/// src/events/mod.rs
use crate::http_client::HttpClient;
use anyhow::Result;
use serde_json::json;

#[derive(Debug)]
pub enum EventStatus {
    NewRun,
    FinishedRun,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::NewRun => "new_run",
            EventStatus::FinishedRun => "finished_run",
        }
    }
}

pub async fn event_pipeline_run_start_new() -> Result<()> {
    let http_client = initialize_http_client().await?;
    println!("Starting new pipeline...");

    log_event(
        &http_client,
        EventStatus::NewRun,
        "[CLI] Starting pipeline run",
    )
    .await?;
    println!("Started pipeline run successfully...");

    Ok(())
}

pub async fn event_pipeline_run_end() -> Result<()> {
    let http_client = initialize_http_client().await?;

    println!("Ending tracer session...");

    log_event(
        &http_client,
        EventStatus::FinishedRun,
        "Pipeline run concluded successfully",
    )
    .await?;
    println!("Ended pipeline run successfully...");

    Ok(())
}

async fn log_event(http_client: &HttpClient, status: EventStatus, message: &str) -> Result<()> {
    let log_entry = json!({
        "message": message,
        "process_type": "pipeline".to_string(),
        "process_status": status.as_str(),
        "event_type": "process_status"
    });

    http_client.send_http_event(&log_entry).await
}

async fn initialize_http_client() -> Result<HttpClient> {
    let service_url = "https://app.tracer.bio/api/data-collector-api".to_string();
    let api_key = "QlXYPyzgjHTipUKUqgr__".to_string();
    let http_client = HttpClient::new(service_url, api_key);
    Ok(http_client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_pipeline_run_start_new() {
        let _ = env_logger::builder().is_test(true).try_init();

        let result = event_pipeline_run_start_new().await;

        assert!(result.is_ok(), "Expected success, but got an error");
    }

    #[tokio::test]
    async fn test_log_event() {
        let _ = env_logger::builder().is_test(true).try_init();

        let service_url = "https://app.tracer.bio/api/data-collector-api".to_string();

        let api_key = "QlXYPyzgjHTipUKUqgr__".to_string();
        let http_client = HttpClient::new(service_url, api_key);
        let message = "[shipping] Test log message from the test suite";

        let result = log_event(&http_client, EventStatus::NewRun, message).await;

        assert!(result.is_ok(), "Expected success, but got an error");
    }

    #[tokio::test]
    async fn test_initialize_http_client() {
        let _ = env_logger::builder().is_test(true).try_init();

        let result = initialize_http_client().await;

        assert!(result.is_ok(), "Expected success, but got an error");
    }

    #[tokio::test]
    async fn test_event_pipeline_run_end() {
        let _ = env_logger::builder().is_test(true).try_init();

        let result = event_pipeline_run_end().await;

        assert!(result.is_ok(), "Expected success, but got an error");
    }
}
