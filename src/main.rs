mod config_manager;
mod event_recorder;
mod events;
mod http_client;
mod metrics;
mod process_watcher;
mod submit_batched_data;
mod tracer_client;

use anyhow::{Context, Result};
use daemonize::Daemonize;
use std::fs::File;
use tokio::time::{sleep, Duration, Instant};

use crate::config_manager::ConfigManager;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";

#[tokio::main]
async fn main() -> Result<()> {
    start_daemon()?;
    run().await
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

pub async fn run() -> Result<()> {
    let config = ConfigManager::load_config().context("Failed to load config")?;
    let mut tracer_client =
        TracerClient::new(config.clone()).context("Failed to create TracerClient")?;

    loop {
        let start_time = Instant::now();
        while start_time.elapsed() < Duration::from_secs(20) {
            monitor_processes_with_tracer_client(&mut tracer_client).await?;
            sleep(Duration::from_millis(config.process_polling_interval_ms)).await;
        }
        tracer_client.submit_batched_data().await?;
    }
}

pub async fn monitor_processes_with_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    tracer_client.remove_completed_processes().await?;
    tracer_client.poll_processes().await?;
    tracer_client.refresh_sysinfo();
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
}
