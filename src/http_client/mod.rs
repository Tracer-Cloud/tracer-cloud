// src/http_client/mod.rs
use anyhow::{Context, Result};
use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct HttpClient {
    client: Client,
    service_url: String,
    api_key: String,
}

impl HttpClient {
    pub fn new(service_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            service_url,
            api_key,
        }
    }

    pub async fn send_http_event(&self, logs: &Value) -> Result<()> {
        // Ensure logs is always an array
        let logs_array = match logs {
            Value::Array(_) => logs.clone(),
            _ => json!([logs]),
        };
        let logs_wrapper = json!({ "logs": logs_array });

        let response = self
            .client
            .post(&self.service_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&logs_wrapper)
            .send()
            .await
            .context("Failed to send event data")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

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
                .open("error_log.txt")
                .await?;
            let log_message = format!(
                "Error while sending send_http_event: {} - {}\n",
                status, response_text
            );
            file.write_all(log_message.as_bytes()).await?;

            Err(anyhow::anyhow!(
                "Error while sending send_http_event: {} - {}",
                status,
                response_text
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_send_http_event() {
        let _ = env_logger::builder().is_test(true).try_init();

        // This should return 200 OK as the server is expected to be running
        // let service_url = "http://localhost:3000/api/data-collector-api".to_string();
        let service_url = "https://app.tracer.bio/api/data-collector-api".to_string();
        let api_key = "_Zx2h6toXUnD1i_QjuRvD".to_string();
        let http_client = HttpClient::new(service_url.clone(), api_key.clone());

        // Define the log data to send
        let logs = json!([
            {
                "message": "starting RNA-seq pipeline RID 255050",
                "process_type": "pipeline",
                "process_status": "new_run",
                "event_type": "process_status"
            }
        ]);

        // Send the HTTP event
        let result = http_client.send_http_event(&logs).await;

        // Ensure the request succeeded
        assert!(result.is_ok(), "Expected success, but got an error");

        if let Err(e) = result {
            assert!(
                !e.to_string().contains("400 Bad Request"),
                "Expected success, but got a 400 Bad Request error: {}",
                e
            );
        }
    }
}
