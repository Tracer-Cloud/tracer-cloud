use crate::setup::paths::ConfigPaths;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc; // Ensure the paths module is correctly imported

#[derive(Serialize, Deserialize)]
pub struct TracerAppConfig {
    pub api_key: String,
    #[serde(default)]
    // This uses the default value for String if not present, which is an empty string
    pub service_url: String,
    #[serde(skip)]
    pub http_client: Arc<Client>,
}

impl TracerAppConfig {
    pub fn load_config() -> Result<Self> {
        let tracer_config_file_path = ConfigPaths::tracer_config_file_path()
            .to_str()
            .unwrap()
            .to_string();

        let config_data = fs::read_to_string(&tracer_config_file_path).with_context(|| {
            format!(
                "Failed to read configuration file at: {}",
                tracer_config_file_path
            )
        })?;

        let mut config: TracerAppConfig = serde_json::from_str(&config_data)
            .with_context(|| "Failed to deserialize the configuration")?;

        config.api_key = config.api_key.trim().to_string();
        config.http_client = Arc::new(Client::new());

        // Load service URL from an environment variable if not provided in the JSON
        if config.service_url.is_empty() {
            config.service_url = std::env::var("TRACER_SERVICE_URL")
                .unwrap_or_else(|_| "https://app.tracer.bio/api/data-collector-api".to_string());
        }

        Ok(config)
    }
}
