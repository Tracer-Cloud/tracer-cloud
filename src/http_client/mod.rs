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

    pub async fn send_log_event (&self, message: String) -> Result<()> {
        let log_entry = json!({
            "message": message,
            "process_type": "pipeline",
            "process_status": "new_run",
            "event_type": "process_status"
        });

        self.send_http_event(&log_entry).await
    }

    pub async fn send_alert_event (&self, message: String) -> Result<()> {
        let alert_entry = json!({
            "message": message,
            "process_type": "pipeline",
            "process_status": "alert",
            "event_type": "process_status"
        });

        self.send_http_event(&alert_entry).await
    }

    pub async fn send_init_event (&self) -> Result<()> {
        let init_entry = json!({
            "message": "Finishing old pipeline run and starting new one",
            "process_type": "pipeline",
            "process_status": "end",
            "event_type": "process_status"
        });

        self.send_http_event(&init_entry).await
    }

    pub fn get_service_url(&self) -> &String {
        &self.service_url
    }

    pub fn get_api_key(&self) -> &String {
        &self.api_key
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
        let http_client = HttpClient::new(service_url, api_key);

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
