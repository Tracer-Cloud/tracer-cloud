use anyhow::{Context, Result};

use crate::{
    config_manager::ConfigManager, events::send_daemon_start_event, PID_FILE, STDERR_FILE,
    STDOUT_FILE,
};

pub fn clean_up_after_daemon() -> Result<()> {
    std::fs::remove_file(PID_FILE).context("Failed to remove pid file")?;
    std::fs::remove_file(STDOUT_FILE).context("Failed to remove stdout file")?;
    std::fs::remove_file(STDERR_FILE).context("Failed to remove stderr file")?;
    Ok(())
}

pub fn print_config_info() -> Result<()> {
    let config = ConfigManager::load_config();
    println!("Service URL: {}", config.service_url);
    println!("API Key: {}", config.api_key);
    println!(
        "Process polling interval: {} ms",
        config.process_polling_interval_ms
    );
    println!(
        "Batch submission interval: {} ms",
        config.batch_submission_interval_ms
    );
    println!("Daemon version: {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

pub fn setup_config(
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
    print_config_info()?;
    println!("Restart the daemon, if running, to apply the new configuration.");
    Ok(())
}

pub async fn test_service_config() -> Result<()> {
    let config = ConfigManager::load_config();

    let result = send_daemon_start_event(&config.service_url, &config.api_key).await;

    if result.is_err() {
        println!("Failed to test the service configuration! Please check the configuration and try again.");
        println!();
        print_config_info()?;
        return result;
    }

    Ok(())
}

pub fn test_service_config_sync() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(test_service_config())
}
