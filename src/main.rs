// src/main.rs
mod config_manager;
mod event_recorder;
mod http_client;
mod metrics;
mod process_watcher;
mod tracer_client;

use anyhow::{Context, Result};
use daemonize::Daemonize;
use std::fs::File;
use tokio::time::{sleep, Duration};

use crate::config_manager::ConfigManager;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";
const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_micros(100); // 0.1 ms in microseconds

#[tokio::main]
async fn main() -> Result<()> {
    start_daemon()?;
    run().await
}

fn start_daemon() -> Result<()> {
    Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(WORKING_DIR)
        .stdout(File::create(STDOUT_FILE).context("Failed to create stdout file")?)
        .stderr(File::create(STDERR_FILE).context("Failed to create stderr file")?)
        .start()
        .context("Failed to start daemon")?;
    println!("tracer-daemon started");
    Ok(())
}

async fn run() -> Result<()> {
    let config = ConfigManager::load_config().context("Failed to load config")?;
    let mut tracer_client =
        TracerClient::from_config(config).context("Failed to create TracerClient")?;

    loop {
        process_tracer_client(&mut tracer_client).await?;
        sleep(DEFAULT_POLLING_INTERVAL).await;
    }
}

async fn process_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    TracerClient::remove_completed_processes(tracer_client).await?;
    TracerClient::poll_processes(tracer_client).await?;
    TracerClient::send_event(tracer_client).await?;
    TracerClient::refresh(tracer_client);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::time::timeout;

    const CONFIG_CONTENT: &str = r#"
        api_key = "_Zx2h6toXUnD1i_QjuRvD"
        polling_interval_ms = 1000
        targets = ["target1", "target2"]
    "#;

    fn create_test_config(content: &str, path: &PathBuf) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config").join("tracer");
        let config_path = config_dir.join("tracer.toml");

        create_test_config(CONFIG_CONTENT, &config_path);

        // Set the HOME environment variable to our temp directory
        env::set_var("HOME", temp_dir.path());

        // Remove TRACER_CONFIG if it's set, to ensure we use the default path
        env::remove_var("TRACER_CONFIG");

        let result = timeout(Duration::from_secs(5), run()).await;
        assert!(
            result.is_err(),
            "run() should not complete within 5 seconds"
        );

        // Clean up is handled automatically by TempDir when it goes out of scope
    }
}
