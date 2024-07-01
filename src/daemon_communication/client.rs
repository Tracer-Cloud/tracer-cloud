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
        polling_interval_ms: Option<u64>,
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

pub async fn send_log_request(socket_path: &str, message: String) {
    let mut socket = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "log",
            "message": message
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket
        .write_all(start_request_json.as_bytes())
        .await
        .expect("Failed to connect to the daemon");
}

pub async fn send_alert_request(socket_path: &str, message: String) {
    let mut socket = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "alert",
            "message": message
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket
        .write_all(start_request_json.as_bytes())
        .await
        .expect("Failed to connect to the daemon");
}

pub async fn send_stop_request(socket_path: &str) {
    let mut socket = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "stop"
    });
    let stop_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket
        .write_all(stop_request_json.as_bytes())
        .await
        .expect("Failed to connect to the daemon");
}

pub async fn send_start_run_request(socket_path: &str) {
    let mut socket = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "start"
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket
        .write_all(start_request_json.as_bytes())
        .await
        .expect("Failed to connect to the daemon");
}

pub async fn send_end_run_request(socket_path: &str) {
    let mut socket = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "end"
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket
        .write_all(start_request_json.as_bytes())
        .await
        .expect("Failed to connect to the daemon");
}
