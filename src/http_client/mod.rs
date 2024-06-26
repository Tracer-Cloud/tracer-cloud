// src/http_client/mod.rs
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

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
        let response = self
            .client
            .post(&self.service_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(logs)
            .send()
            .await
            .context("Failed to send event data")?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Error while sending metrics: {}",
                response.status()
            ))
        }
    }
}
