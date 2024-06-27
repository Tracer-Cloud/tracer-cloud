// src/main.rs
mod config_manager;
mod event_recorder;
mod events;
mod http_client;
mod metrics;
mod process_watcher;
mod tracer_client;

use anyhow::{Context, Result};
use daemonize::Daemonize;
use std::fs::File;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, sleep, Duration};

use crate::config_manager::ConfigManager;
use crate::events::event_pipeline_run_start_new;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";

#[tokio::main]
async fn main() -> Result<()> {
    start_daemon().await?;
    run().await
}

async fn start_daemon() -> Result<()> {
    Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(WORKING_DIR)
        .stdout(File::create(STDOUT_FILE).context("Failed to create stdout file")?)
        .stderr(File::create(STDERR_FILE).context("Failed to create stderr file")?)
        .start()
        .context("Failed to start daemon")?;
    println!("tracer-daemon started");
    // Start new pipeline run event
    event_pipeline_run_start_new().await?;
    Ok(())
}

async fn run() -> Result<()> {
    let config = ConfigManager::load_config().context("Failed to load config")?;
    let tracer_client = Arc::new(Mutex::new(
        TracerClient::new(config.clone()).context("Failed to create TracerClient")?,
    ));
    let (tx, rx) = mpsc::channel::<()>(1);

    spawn_batch_submission_task(
        Arc::clone(&tracer_client),
        rx,
        config.batch_submission_interval_ms,
    );

    loop {
        monitor_processes_with_tracer_client(&tracer_client, &tx).await?;
        sleep(Duration::from_millis(config.process_polling_interval_ms)).await;
    }
}

fn spawn_batch_submission_task(
    tracer_client: Arc<Mutex<TracerClient>>,
    mut rx: mpsc::Receiver<()>,
    interval_ms: u64,
) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            if rx.recv().await.is_some() {
                let mut tracer_client = tracer_client.lock().await;
                if let Err(e) = tracer_client.submit_batched_data().await {
                    eprintln!("Failed to submit batched data: {}", e);
                }
            }
        }
    });
}

async fn monitor_processes_with_tracer_client(
    tracer_client: &Arc<Mutex<TracerClient>>,
    tx: &mpsc::Sender<()>,
) -> Result<()> {
    let mut tracer_client = tracer_client.lock().await;
    tracer_client.remove_completed_processes().await?;
    tracer_client.poll_processes().await?;

    if tx.send(()).await.is_err() {
        eprintln!("Failed to send signal for batch submission");
    }

    tracer_client.refresh();
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
        api_key = "test_api_key"
        process_polling_interval_ms = 200
        batch_submission_interval_ms = 5000
        service_url = "https://app.tracer.bio/api/data-collector-api"
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

        env::set_var("HOME", temp_dir.path());
        env::remove_var("TRACER_CONFIG");

        let result = timeout(Duration::from_secs(5), run()).await;
        assert!(
            result.is_err(),
            "run() should not complete within 5 seconds"
        );
    }
}
