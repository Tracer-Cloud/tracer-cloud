use std::process::Command;

use anyhow::{Context, Ok, Result};

use crate::{
    config_manager::ConfigManager,
    daemon_communication::client::{send_ping_request, send_refresh_config_request},
    events::send_daemon_start_event,
    PID_FILE, REPO_NAME, REPO_OWNER, SOCKET_PATH, STDERR_FILE, STDOUT_FILE,
};

pub fn clean_up_after_daemon() -> Result<()> {
    std::fs::remove_file(PID_FILE).context("Failed to remove pid file")?;
    std::fs::remove_file(STDOUT_FILE).context("Failed to remove stdout file")?;
    std::fs::remove_file(STDERR_FILE).context("Failed to remove stderr file")?;
    Ok(())
}

pub async fn print_config_info() -> Result<()> {
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
    let daemon_status = send_ping_request(SOCKET_PATH).await;
    if daemon_status.is_ok() {
        println!("Daemon status: Running");
    } else {
        println!("Daemon status: Stopped");
    }
    Ok(())
}

pub fn print_config_info_sync() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(print_config_info())?;
    Ok(())
}

pub async fn setup_config(
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
    let _ = send_refresh_config_request(SOCKET_PATH).await;
    print_config_info().await?;
    Ok(())
}

pub async fn update_tracer() -> Result<()> {
    let octocrab = octocrab::instance();

    let release = octocrab
        .repos(REPO_OWNER, REPO_NAME)
        .releases()
        .get_latest()
        .await?;

    if release.tag_name == env!("CARGO_PKG_VERSION") {
        println!("You are already using the latest version of Tracer.");
        return Ok(());
    }

    let config = ConfigManager::load_config();

    println!("Updating Tracer to version {}", release.tag_name);

    let mut command = Command::new("bash");
    command.arg("-c").arg(format!("curl -sSL https://raw.githubusercontent.com/davincios/tracer-daemon/main/install-tracer.sh | bash -s -- {} && . ~/.bashrc && tracer", config.api_key));

    command
        .status()
        .context("Failed to update Tracer. Please try again.")?;

    Ok(())
}

pub async fn test_service_config() -> Result<()> {
    let config = ConfigManager::load_config();

    let result = send_daemon_start_event(&config.service_url, &config.api_key).await;

    if result.is_err() {
        println!("Failed to test the service configuration! Please check the configuration and try again.");
        println!("{}", result.as_ref().unwrap_err());
        print_config_info().await?;
        return result;
    }

    Ok(())
}

pub fn test_service_config_sync() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(test_service_config())
}
