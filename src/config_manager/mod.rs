use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
mod targets;

const DEFAULT_API_KEY: &str = "dIdd4HI9ixcQtw7xsulnv";
const DEFAULT_SERVICE_URL: &str = "https://app.tracer.bio/api/data-collector-api";
const DEFAULT_CONFIG_FILE_LOCATION_FROM_HOME: &str = ".config/tracer/tracer.toml";
// const DEFAULT_SERVICE_URL: &str = "http://localhost:3000/api/data-collector-api";

const PROCESS_POLLING_INTERVAL_MS: u64 = 10;
const BATCH_SUBMISSION_INTERVAL_MS: u64 = 5000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigFile {
    pub api_key: String,
    pub process_polling_interval_ms: u64,
    pub batch_submission_interval_ms: u64,
    pub service_url: String,
    pub targets: Vec<String>,
}

pub struct ConfigManager;

impl ConfigManager {
    fn get_config_path() -> Option<PathBuf> {
        let path = homedir::get_my_home();

        match path {
            Ok(Some(path)) => {
                let path = path.join(DEFAULT_CONFIG_FILE_LOCATION_FROM_HOME);
                Some(path)
            }
            _ => None,
        }
    }

    fn load_config_from_file(path: &PathBuf) -> Result<ConfigFile> {
        let config = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&config)?;
        Ok(config)
    }

    fn load_default_config() -> ConfigFile {
        let config = ConfigFile {
            api_key: DEFAULT_API_KEY.to_string(),
            process_polling_interval_ms: PROCESS_POLLING_INTERVAL_MS,
            batch_submission_interval_ms: BATCH_SUBMISSION_INTERVAL_MS,
            service_url: DEFAULT_SERVICE_URL.to_string(),
            targets: targets::TARGETS.iter().map(|&s| s.to_string()).collect(),
        };
        config
    }

    pub fn load_config() -> ConfigFile {
        let config_file_location = ConfigManager::get_config_path();

        let config = if let Some(path) = config_file_location {
            ConfigManager::load_config_from_file(&path)
                .unwrap_or_else(|_| ConfigManager::load_default_config())
        } else {
            ConfigManager::load_default_config()
        };

        let api_key = std::env::var("TRACER_API_KEY").unwrap_or_else(|_| config.api_key.clone());

        let service_url =
            std::env::var("TRACER_SERVICE_URL").unwrap_or_else(|_| config.service_url.clone());

        ConfigFile {
            api_key,
            service_url,
            ..config
        }
    }

    pub fn save_config(config: &ConfigFile) -> Result<()> {
        let config_file_location = ConfigManager::get_config_path().unwrap();
        let config = toml::to_string(config)?;
        std::fs::write(config_file_location, config)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        env::remove_var("TRACER_API_KEY");
        env::remove_var("TRACER_SERVICE_URL");
        let config = ConfigManager::load_config();
        assert_eq!(config.api_key, DEFAULT_API_KEY);
        assert_eq!(config.service_url, DEFAULT_SERVICE_URL);
        assert_eq!(
            config.process_polling_interval_ms,
            PROCESS_POLLING_INTERVAL_MS
        );
        assert_eq!(
            config.batch_submission_interval_ms,
            BATCH_SUBMISSION_INTERVAL_MS
        );
        assert!(!config.targets.is_empty());
    }
}
