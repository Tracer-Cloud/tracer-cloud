use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
mod targets;

const DEFAULT_API_KEY: &str = "EAjg7eHtsGnP3fTURcPz1";
const DEFAULT_SERVICE_URL: &str = "https://app.tracer.bio/api/data-collector-api";
const DEFAULT_CONFIG_FILE_LOCATION_FROM_HOME: &str = ".config/tracer/tracer.toml";
const PROCESS_POLLING_INTERVAL_MS: u64 = 50;
const BATCH_SUBMISSION_INTERVAL_MS: u64 = 10000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandContainsStruct {
    command_content: String,
    merge_with_parents: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Target {
    ProcessName(String),
    ShortLivedProcessExecutable(String),
    CommandContains(CommandContainsStruct),
}

impl Target {
    pub fn matches(&self, process_name: &str, command: &str) -> bool {
        match self {
            Target::ProcessName(name) => process_name == name,
            Target::ShortLivedProcessExecutable(_) => false,
            Target::CommandContains(inner) => command.contains(&inner.command_content),
        }
    }

    pub fn should_be_merged_with_parents(&self) -> bool {
        match self {
            Target::ProcessName(_) => false,
            Target::ShortLivedProcessExecutable(_) => false,
            Target::CommandContains(inner) => inner.merge_with_parents,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigFile {
    pub api_key: String,
    pub service_url: Option<String>,
    pub process_polling_interval_ms: Option<u64>,
    pub batch_submission_interval_ms: Option<u64>,
    pub targets: Option<Vec<Target>>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub process_polling_interval_ms: u64,
    pub batch_submission_interval_ms: u64,
    pub service_url: String,
    pub targets: Vec<Target>,
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

    fn load_config_from_file(path: &PathBuf) -> Result<Config> {
        let config = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&config)?;
        Ok(Config {
            api_key: config.api_key,
            process_polling_interval_ms: config
                .process_polling_interval_ms
                .unwrap_or(PROCESS_POLLING_INTERVAL_MS),
            batch_submission_interval_ms: config
                .batch_submission_interval_ms
                .unwrap_or(BATCH_SUBMISSION_INTERVAL_MS),
            service_url: config
                .service_url
                .unwrap_or(DEFAULT_SERVICE_URL.to_string()),
            targets: config.targets.unwrap_or_else(|| targets::TARGETS.to_vec()),
        })
    }

    pub fn load_default_config() -> Config {
        Config {
            api_key: DEFAULT_API_KEY.to_string(),
            process_polling_interval_ms: PROCESS_POLLING_INTERVAL_MS,
            batch_submission_interval_ms: BATCH_SUBMISSION_INTERVAL_MS,
            service_url: DEFAULT_SERVICE_URL.to_string(),
            targets: targets::TARGETS.to_vec(),
        }
    }

    pub fn load_config() -> Config {
        let config_file_location = ConfigManager::get_config_path();

        let mut config = if let Some(path) = config_file_location {
            let loaded_config = ConfigManager::load_config_from_file(&path);
            if loaded_config.is_err() {
                println!(
                    "\nFailed to load config from {:?}, using default config.\n",
                    path
                )
            }
            loaded_config.unwrap_or_else(|_| ConfigManager::load_default_config())
        } else {
            ConfigManager::load_default_config()
        };

        if let Ok(api_key) = std::env::var("TRACER_API_KEY") {
            config.api_key = api_key;
        }

        if let Ok(service_url) = std::env::var("TRACER_SERVICE_URL") {
            config.service_url = service_url;
        }

        config
    }

    pub fn save_config(config: &Config) -> Result<()> {
        let config_file_location = ConfigManager::get_config_path().unwrap();
        let config_out = ConfigFile {
            api_key: config.api_key.clone(),
            service_url: Some(config.service_url.clone()),
            process_polling_interval_ms: Some(config.process_polling_interval_ms),
            batch_submission_interval_ms: Some(config.batch_submission_interval_ms),
            targets: Some(config.targets.clone()),
        };
        let config = toml::to_string(&config_out)?;
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
        let config = ConfigManager::load_default_config();
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
