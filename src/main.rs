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
// use crate::events::event_pipeline_run_start_new;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";

// please provide me the std out file of tracer:
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
    // event_pipeline_run_start_new().await?;
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
