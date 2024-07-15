mod cli;
mod config_manager;
mod daemon_communication;
mod event_recorder;
mod events;
mod http_client;
mod metrics;
mod nondaemon_commands;
mod process_watcher;
mod submit_batched_data;
mod task_wrapper;
mod tracer_client;

use anyhow::{Context, Ok, Result};
use cli::process_cli;
use daemon_communication::server::run_server;
use daemonize::Daemonize;
use nondaemon_commands::test_service_config_sync;
use std::borrow::BorrowMut;

use std::fs::File;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::config_manager::ConfigManager;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";
const SOCKET_PATH: &str = "/tmp/tracerd.sock";

const REPO_OWNER: &str = "davincios";
const REPO_NAME: &str = "tracer-daemon";

pub fn start_daemon() -> Result<()> {
    test_service_config_sync()?;

    let daemon = Daemonize::new();
    daemon
        .pid_file(PID_FILE)
        .working_directory(WORKING_DIR)
        .stdout(
            File::create(STDOUT_FILE)
                .context("Failed to create stdout file")
                .unwrap(),
        )
        .stderr(
            File::create(STDERR_FILE)
                .context("Failed to create stderr file")
                .unwrap(),
        )
        .start()
        .context("Failed to start daemon.")
}

pub fn main() -> Result<()> {
    process_cli()
}

#[tokio::main]
pub async fn run() -> Result<()> {
    let raw_config = ConfigManager::load_config();
    let client = TracerClient::new(raw_config.clone()).context("Failed to create TracerClient")?;
    let tracer_client = Arc::new(Mutex::new(client));
    let config: Arc<RwLock<config_manager::Config>> = Arc::new(RwLock::new(raw_config));

    let cancellation_token = CancellationToken::new();
    tokio::spawn(run_server(
        tracer_client.clone(),
        SOCKET_PATH,
        cancellation_token.clone(),
        config.clone(),
    ));

    while !cancellation_token.is_cancelled() {
        let start_time = Instant::now();
        while start_time.elapsed()
            < Duration::from_millis(config.read().await.batch_submission_interval_ms)
        {
            monitor_processes_with_tracer_client(tracer_client.lock().await.borrow_mut()).await?;
            sleep(Duration::from_millis(
                config.read().await.process_polling_interval_ms,
            ))
            .await;
            if cancellation_token.is_cancelled() {
                break;
            }
        }

        tracer_client
            .lock()
            .await
            .borrow_mut()
            .submit_batched_data()
            .await?;
    }

    Ok(())
}

pub async fn monitor_processes_with_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    tracer_client.remove_completed_processes().await?;
    tracer_client.poll_processes().await?;
    tracer_client.poll_process_metrics().await?;
    tracer_client.refresh_sysinfo();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use config_manager::Config;

    fn load_test_config() -> Config {
        ConfigManager::load_default_config()
    }

    #[tokio::test]
    async fn test_monitor_processes_with_tracer_client() {
        let config = load_test_config();
        let mut tracer_client = TracerClient::new(config).unwrap();
        let result = monitor_processes_with_tracer_client(&mut tracer_client).await;
        assert!(result.is_ok());
    }
}
