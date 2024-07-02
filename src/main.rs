mod config_manager;
mod daemon_communication;
mod event_recorder;
mod events;
mod http_client;
mod metrics;
mod nondaemon_commands;
mod process_watcher;
mod submit_batched_data;
mod tracer_client;

use anyhow::{Context, Ok, Result};
use clap::Parser;
use daemon_communication::client::{
    send_alert_request, send_end_run_request, send_log_request, send_start_run_request,
    send_stop_request, Cli, Commands,
};
use daemon_communication::server::run_server;
use daemonize::Daemonize;
use nondaemon_commands::{
    clean_up_after_daemon, print_config_info, setup_config, test_service_config_sync,
};
use std::borrow::BorrowMut;
use std::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::config_manager::ConfigManager;
use crate::tracer_client::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";
const SOCKET_PATH: &str = "/tmp/tracerd.sock";

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

#[tokio::main]
pub async fn run_daemon_client_command(commands: Commands) -> Result<()> {
    let value = match commands {
        Commands::Log { message } => send_log_request(SOCKET_PATH, message).await,
        Commands::Alert { message } => send_alert_request(SOCKET_PATH, message).await,
        Commands::Stop => send_stop_request(SOCKET_PATH).await,
        Commands::Start => send_start_run_request(SOCKET_PATH).await,
        Commands::End => send_end_run_request(SOCKET_PATH).await,
        _ => {
            println!("Command not implemented yet");
            Ok(())
        }
    };

    if value.is_err() {
        println!("Failed to send command to the daemon. Maybe the daemon is not running? If it's not, run `tracer init` to start the daemon.");
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            let test_result = test_service_config_sync();
            if test_result.is_err() {
                return Ok(());
            }
            let result = start_daemon();
            if result.is_err() {
                println!("Failed to start daemon. Maybe the daemon is already running? If it's not, run `tracer cleanup` to clean up the previous daemon files.");
                return Ok(());
            }
            run()?;
            clean_up_after_daemon()
        }
        Commands::Setup {
            api_key,
            service_url,
            process_polling_interval_ms,
            batch_submission_interval_ms,
        } => setup_config(
            api_key,
            service_url,
            process_polling_interval_ms,
            batch_submission_interval_ms,
        ),
        Commands::Test => {
            let _ = test_service_config_sync();
            Ok(())
        }
        Commands::Cleanup => clean_up_after_daemon(),
        Commands::Info => print_config_info(),
        _ => run_daemon_client_command(cli.command),
    }
}

#[tokio::main]
pub async fn run() -> Result<()> {
    let config = ConfigManager::load_config();
    let client = TracerClient::new(config.clone()).context("Failed to create TracerClient")?;
    let tracer_client = Arc::new(Mutex::new(client));

    let cancellation_token = CancellationToken::new();
    tokio::spawn(run_server(
        tracer_client.clone(),
        SOCKET_PATH,
        cancellation_token.clone(),
    ));

    while !cancellation_token.is_cancelled() {
        let start_time = Instant::now();
        while start_time.elapsed() < Duration::from_millis(config.batch_submission_interval_ms) {
            monitor_processes_with_tracer_client(tracer_client.lock().await.borrow_mut()).await?;
            sleep(Duration::from_millis(config.process_polling_interval_ms)).await;
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
    tracer_client.refresh_sysinfo();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use config_manager::ConfigFile;

    fn load_test_config() -> ConfigFile {
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
