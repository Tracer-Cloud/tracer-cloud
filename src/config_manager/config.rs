// src/config_manager/mod.rs
use std::{env, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    config_manager::{
        bashrc_intercept::{modify_bashrc_file, rewrite_interceptor_bashrc_file},
        target_process::target_matching::TargetMatch,
    },
    events::send_daemon_start_event,
};

use crate::config_manager::target_process::Target;

use super::target_process::targets_list;

const DEFAULT_API_KEY: &str = "EAjg7eHtsGnP3fTURcPz1";
const DEFAULT_SERVICE_URL: &str = "https://app.tracer.bio/api/data-collector-api";
const DEFAULT_CONFIG_FILE_LOCATION_FROM_HOME: &str = ".config/tracer/tracer.toml";
const PROCESS_POLLING_INTERVAL_MS: u64 = 50;
const BATCH_SUBMISSION_INTERVAL_MS: u64 = 10000;
const NEW_RUN_PAUSE_MS: u64 = 10 * 60 * 1000;
const PROCESS_METRICS_SEND_INTERVAL_MS: u64 = 10000;
const FILE_SIZE_NOT_CHANGING_PERIOD_MS: u64 = 1000 * 60;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigFile {
    pub api_key: String,
    pub service_url: Option<String>,
    pub process_polling_interval_ms: Option<u64>,
    pub batch_submission_interval_ms: Option<u64>,
    pub new_run_pause_ms: Option<u64>,
    pub file_size_not_changing_period_ms: Option<u64>,
    pub process_metrics_send_interval_ms: Option<u64>,
    pub targets: Option<Vec<Target>>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub process_polling_interval_ms: u64,
    pub batch_submission_interval_ms: u64,
    pub process_metrics_send_interval_ms: u64,
    pub file_size_not_changing_period_ms: u64,
    pub service_url: String,
    pub new_run_pause_ms: u64,
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
            new_run_pause_ms: config.new_run_pause_ms.unwrap_or(NEW_RUN_PAUSE_MS),
            process_metrics_send_interval_ms: config
                .process_metrics_send_interval_ms
                .unwrap_or(PROCESS_METRICS_SEND_INTERVAL_MS),
            file_size_not_changing_period_ms: config
                .file_size_not_changing_period_ms
                .unwrap_or(FILE_SIZE_NOT_CHANGING_PERIOD_MS),
            targets: config
                .targets
                .unwrap_or_else(|| targets_list::TARGETS.to_vec()),
        })
    }

    pub fn load_default_config() -> Config {
        Config {
            api_key: DEFAULT_API_KEY.to_string(),
            process_polling_interval_ms: PROCESS_POLLING_INTERVAL_MS,
            batch_submission_interval_ms: BATCH_SUBMISSION_INTERVAL_MS,
            new_run_pause_ms: NEW_RUN_PAUSE_MS,
            file_size_not_changing_period_ms: FILE_SIZE_NOT_CHANGING_PERIOD_MS,
            service_url: DEFAULT_SERVICE_URL.to_string(),
            targets: targets_list::TARGETS.to_vec(),
            process_metrics_send_interval_ms: PROCESS_METRICS_SEND_INTERVAL_MS,
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

    pub fn setup_aliases() -> Result<()> {
        let config = ConfigManager::load_config();
        rewrite_interceptor_bashrc_file(
            env::current_exe()?,
            config
                .targets
                .iter()
                .filter(|target| {
                    matches!(
                        &target.match_type,
                        TargetMatch::ShortLivedProcessExecutable(_)
                    )
                })
                .collect(),
        )?;
        // bashrc_intercept(".bashrc")?;
        modify_bashrc_file(".bashrc")?;

        println!("Command interceptors setup successfully.");
        Ok(())
    }

    pub fn save_config(config: &Config) -> Result<()> {
        let config_file_location = ConfigManager::get_config_path().unwrap();
        let config_out = ConfigFile {
            api_key: config.api_key.clone(),
            service_url: Some(config.service_url.clone()),
            new_run_pause_ms: Some(config.new_run_pause_ms),
            file_size_not_changing_period_ms: Some(config.file_size_not_changing_period_ms),
            process_polling_interval_ms: Some(config.process_polling_interval_ms),
            batch_submission_interval_ms: Some(config.batch_submission_interval_ms),
            targets: Some(config.targets.clone()),
            process_metrics_send_interval_ms: Some(config.process_metrics_send_interval_ms),
        };
        let config = toml::to_string(&config_out)?;
        std::fs::write(config_file_location, config)?;
        Ok(())
    }

    pub fn modify_config(
        api_key: &Option<String>,
        service_url: &Option<String>,
        process_polling_interval_ms: &Option<u64>,
        batch_submission_interval_ms: &Option<u64>,
    ) -> Result<()> {
        let mut current_config = ConfigManager::load_config();
        if let Some(api_key) = api_key {
            current_config.api_key.clone_from(api_key);
        }
        if let Some(service_url) = service_url {
            current_config.service_url.clone_from(service_url);
        }
        if let Some(process_polling_interval_ms) = process_polling_interval_ms {
            current_config.process_polling_interval_ms = *process_polling_interval_ms;
        }
        if let Some(batch_submission_interval_ms) = batch_submission_interval_ms {
            current_config.batch_submission_interval_ms = *batch_submission_interval_ms;
        }
        ConfigManager::save_config(&current_config)?;
        Ok(())
    }

    pub async fn test_service_config() -> Result<()> {
        let config = ConfigManager::load_config();

        let result = send_daemon_start_event(&config.service_url, &config.api_key).await;

        if let Err(error) = result {
            println!("Failed to test the service configuration! Please check the configuration and try again.");
            println!("{}", &error);
            return Err(error);
        }

        Ok(())
    }

    pub fn test_service_config_sync() -> Result<()> {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(ConfigManager::test_service_config())
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
        assert_eq!(
            config.process_metrics_send_interval_ms,
            PROCESS_METRICS_SEND_INTERVAL_MS
        );
        assert!(!config.targets.is_empty());
    }
}
