/// lib.rs
pub mod cloud_providers;
pub mod config_manager;
pub mod daemon_communication;
pub mod debug_log;
pub mod event_recorder;
pub mod events;
pub mod exporters;
pub mod file_watcher;
pub mod http_client;
pub mod metrics;
pub mod process_watcher;
pub mod stdout;
pub mod submit_batched_data;
pub mod syslog;
pub mod tracer_client;
pub mod tracing;
pub mod types;
pub mod upload;
pub mod cli;

use anyhow::{Context, Ok, Result};
use config_manager::{INTERCEPTOR_STDERR_FILE, INTERCEPTOR_STDOUT_FILE};
use daemon_communication::server::run_server;
use daemonize::Daemonize;
use std::borrow::BorrowMut;
use syslog::run_syslog_lines_read_thread;

use crate::exporters::{FsExportHandler, S3ExportHandler};
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
const FILE_CACHE_DIR: &str = "/tmp/tracerd_cache";

const SYSLOG_FILE: &str = "/var/log/syslog";

const REPO_OWNER: &str = "davincios";
const REPO_NAME: &str = "tracer-daemon";

pub fn start_daemon() -> Result<()> {
    ConfigManager::test_service_config_sync()?;

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

#[tokio::main]
pub async fn run(workflow_directory_path: String) -> Result<()> {
    let raw_config = ConfigManager::load_config();

    let export_dir = ConfigManager::get_tracer_parquet_export_dir()?;

    let fs_handler = FsExportHandler::new(export_dir, None);
    let s3_handler = S3ExportHandler::new(
        fs_handler,
        raw_config.aws_init_type.clone(),
        raw_config.aws_region.as_str(),
    )
    .await;

    let client = TracerClient::new(raw_config.clone(), workflow_directory_path, s3_handler)
        .await
        .context("Failed to create TracerClient")?;
    let tracer_client = Arc::new(Mutex::new(client));
    let config: Arc<RwLock<config_manager::Config>> = Arc::new(RwLock::new(raw_config));

    let cancellation_token = CancellationToken::new();

    tokio::spawn(run_server(
        tracer_client.clone(),
        SOCKET_PATH,
        cancellation_token.clone(),
        config.clone(),
    ));

    let syslog_lines_task = tokio::spawn(run_syslog_lines_read_thread(
        SYSLOG_FILE,
        tracer_client.lock().await.get_syslog_lines_buffer(),
    ));

    let stdout_lines_task = tokio::spawn(stdout::run_stdout_lines_read_thread(
        INTERCEPTOR_STDOUT_FILE,
        INTERCEPTOR_STDERR_FILE,
        tracer_client.lock().await.get_stdout_stderr_lines_buffer(),
    ));

    tracer_client
        .lock()
        .await
        .borrow_mut()
        .start_new_run(None)
        .await?;

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

        tracer_client.lock().await.borrow_mut().poll_files().await?;
    }

    syslog_lines_task.abort();
    stdout_lines_task.abort();

    Ok(())
}

pub async fn monitor_processes_with_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    tracer_client.remove_completed_processes().await?;
    tracer_client.poll_processes()?;
    // tracer_client.run_cleanup().await?;
    tracer_client.poll_process_metrics().await?;
    tracer_client.poll_syslog().await?;
    tracer_client.poll_stdout_stderr().await?;
    tracer_client.refresh_sysinfo();
    tracer_client.reset_just_started_process_flag();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use aws_config::BehaviorVersion;
    use config_manager::Config;

    fn load_test_config() -> Config {
        ConfigManager::load_default_config()
    }

    #[tokio::test]
    async fn test_monitor_processes_with_tracer_client() {
        let config = load_test_config();
        let pwd = std::env::current_dir().unwrap();

        let region = "us-east-2";
        crate::cloud_providers::aws::setup_env_vars("us-east-2");

        // TODO: Might have to move this into config
        let mut base_dir = homedir::get_my_home()
            .unwrap()
            .expect("Failed to get home dir");
        base_dir.push("exports");

        let fs_handler = FsExportHandler::new(base_dir, None);

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").unwrap();

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint_url.clone())
            .load()
            .await;

        let s3_handler = S3ExportHandler::new_with_config(fs_handler, aws_config).await;

        let mut tracer_client =
            TracerClient::new(config, pwd.to_str().unwrap().to_string(), s3_handler)
                .await
                .unwrap();
        let result = monitor_processes_with_tracer_client(&mut tracer_client).await;
        assert!(result.is_ok());
    }
}
