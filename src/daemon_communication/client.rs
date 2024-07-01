// src/cli.rs
use clap::{Parser, Subcommand};
use serde_json::json;
use tokio::{io::AsyncWriteExt, net::UnixStream};

#[derive(Parser)]
#[clap(
    name = "tracer",
    about = "A tool for monitoring bioinformatics applications",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Setup {
        api_key: Option<String>,
        service_url: Option<String>,
        polling_interval_us: Option<u64>,
        batch_submission_interval_ms: Option<u64>,
    },
    Log {
        message: String,
    },
    Alert {
        message: String,
    },
    Init,
    Cleanup,
    Info,
    Stop,
    Update,
    Start,
    End,
    Version,
}

pub async fn send_log_request(socket_path: &str, message: String) -> Result<(), anyhow::Error> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let log_request = json!({
            "command": "log",
            "message": message
    });
    let log_request_json =
        serde_json::to_string(&log_request).expect("Failed to serialize log request");
    socket.write_all(log_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_alert_request(socket_path: &str, message: String) -> Result<(), anyhow::Error> {
    let mut socket = UnixStream::connect(socket_path).await?;
    let alert_request = json!({
            "command": "alert",
            "message": message
    });
    let alert_request_json =
        serde_json::to_string(&alert_request).expect("Failed to serialize alrt request");
    socket.write_all(alert_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_stop_request(socket_path: &str) -> Result<(), anyhow::Error> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let stop_request = json!({
            "command": "stop"
    });

    let stop_request_json =
        serde_json::to_string(&stop_request).expect("Failed to serialize stop request");

    socket.write_all(stop_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_start_run_request(socket_path: &str) -> Result<(), anyhow::Error> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let start_request = json!({
            "command": "start"
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket.write_all(start_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_end_run_request(socket_path: &str) -> Result<(), anyhow::Error> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let end_request = json!({
            "command": "end"
    });

    let end_request_json =
        serde_json::to_string(&end_request).expect("Failed to serialize start request");

    socket.write_all(end_request_json.as_bytes()).await?;

    Ok(())
}
