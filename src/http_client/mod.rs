use anyhow::{Context, Ok, Result};
use chrono::Utc;
use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

/// Logs all outgoing HTTP calls to a file.
async fn record_all_outgoing_http_calls(
    service_url: &str,
    api_key: &str,
    request_body: &Value,
) -> Result<()> {
    // Log the request body to a log file so that we can test WHAT and IF there are any outgoing messages
    let timestamp = Utc::now().to_rfc3339();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("log_outgoing_http_calls.txt")
        .await?;

    let incoming_logs_string = format!(
        "[{}] send_http_event: {} - {}\nRequest body: {}\n----------\n",
        timestamp,
        api_key,
        service_url,
        request_body, // Convert request_body to string
    );
    file.write_all(incoming_logs_string.as_bytes()).await?;
    Ok(())
}

pub async fn send_http_event(service_url: &str, api_key: &str, logs: &Value) -> Result<()> {
    // Log request body
    let logs_array = match logs {
        Value::Array(_) => logs.clone(),
        _ => json!([logs]),
    };
    let request_body = json!({ "logs": logs_array });
    record_all_outgoing_http_calls(service_url, api_key, &request_body).await?;

    // Send request
    let client = Client::new();
    let response = client
        .post(service_url)
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send event data")?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

    // Log response body
    info!(
        "Response status: {}, Response body: {}",
        status, response_text
    );

    if status.is_success() {
        info!(
            "Successfully sent HTTP event: {} - {}",
            status, response_text
        );
        Ok(())
    } else {
        error!(
            "Error while sending send_http_event: {} - {}",
            status, response_text
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("error_outgoing_http_calls.txt")
            .await?;
        let log_message = format!(
            "Error while sending send_http_event: {} - {}\nRequest body: {}\nResponse body: {}\n",
            status, response_text, request_body, response_text
        );
        file.write_all(log_message.as_bytes()).await?;

        Err(anyhow::anyhow!(
            "Error while sending send_http_event: {} - {}",
            status,
            response_text
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Error;
    use serde_json::json;

    #[tokio::test]
    async fn test_send_http_event() -> Result<(), Error> {
        let _ = env_logger::builder().is_test(true).try_init();

        // Load configuration
        let config = ConfigManager::load_config().context("Failed to load config")?;
        let api_key = config.api_key.clone(); // Cloning here to avoid moving
        let service_url = config.service_url.clone(); // Cloning here to avoid moving

        // Define the log data to send
        let logs = json!([
            {
                "message": "[test_send_http_event] starting RNA-seq pipeline RID 255050",
                "process_type": "pipeline",
                "process_status": "new_run",
                "event_type": "process_status"
            }
        ]);

        // Send the HTTP event
        let result = send_http_event(&service_url, &api_key, &logs).await;

        // Ensure the request succeeded
        assert!(
            result.is_ok(),
            "Expected success, but got an error: {:?}",
            result
        );

        if let Err(e) = result {
            assert!(
                !e.to_string().contains("400 Bad Request"),
                "Expected success, but got a 400 Bad Request error: {}",
                e
            );
        }

        Ok(())
    }
}
