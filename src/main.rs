mod config_manager;
mod daemon_communication;
mod data_submission;
mod event_recorder;
mod events;
mod http_client;
mod metrics;
mod process_watcher;
mod tracer_client;

use anyhow::{Context, Result};
use daemon_communication::client::parse_input;
use daemon_communication::server::run_server;
use daemonize::Daemonize;
use std::borrow::BorrowMut;
use std::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

use crate::config_manager::ConfigManager;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";
const SOCKET_PATH: &str = "/tmp/tracerd.sock";

fn main() -> Result<()> {
    let deamon_status = start_daemon();
    if deamon_status.is_ok() {
        run()
    } else {
        run_cli()
    }
}

pub fn start_daemon() -> Result<()> {
    Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(WORKING_DIR)
        .stdout(File::create(STDOUT_FILE).context("Failed to create stdout file")?)
        .stderr(File::create(STDERR_FILE).context("Failed to create stderr file")?)
        .start()
        .context("Failed to start daemon")?;
    println!("tracer started");
    Ok(())
}

#[tokio::main]
pub async fn run_cli() -> Result<()> {
    parse_input(SOCKET_PATH).await;
    Ok(())
}

#[tokio::main]
pub async fn run() -> Result<()> {
    let config = ConfigManager::load_config().context("Failed to load config")?;
    let client = TracerClient::new(config.clone()).context("Failed to create TracerClient")?;
    let tracer_client = Arc::new(Mutex::new(client));

    tokio::spawn(run_server(tracer_client.clone(), SOCKET_PATH));

    loop {
        let start_time = Instant::now();
        while start_time.elapsed() < Duration::from_secs(20) {
            monitor_processes_with_tracer_client(tracer_client.lock().await.borrow_mut()).await?;
            sleep(Duration::from_millis(config.process_polling_interval_ms)).await;
        }
        submit_metrics(tracer_client.lock().await.borrow_mut()).await?;
    }
}

pub async fn monitor_processes_with_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    tracer_client.remove_completed_processes().await?;
    tracer_client.poll_processes().await?;
    tracer_client.refresh();
    Ok(())
}

pub async fn submit_metrics(tracer_client: &mut TracerClient) -> Result<()> {
    if let Err(e) = tracer_client.submit_batched_data().await {
        eprintln!("Failed to submit batched data: {}", e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use anyhow::Context;
    use config_manager::ConfigFile;

    fn load_test_config() -> ConfigFile {
        ConfigManager::load_config()
            .context("Failed to load config")
            .unwrap()
    }

    #[tokio::test]
    async fn test_monitor_processes_with_tracer_client() {
        let config = load_test_config();
        let mut tracer_client = TracerClient::new(config).unwrap();
        let result = monitor_processes_with_tracer_client(&mut tracer_client).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_submit_metrics() {
        let config = load_test_config();
        let mut tracer_client = TracerClient::new(config).unwrap();
        let result = submit_metrics(&mut tracer_client).await;
        assert!(result.is_ok());
    }
}
