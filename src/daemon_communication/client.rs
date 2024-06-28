// src/cli.rs
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
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

#[derive(Subcommand)]
pub enum Commands {
    Setup { api_key: String },
    Log { message: String },
    Alert { message: String },
    Init,
    Update,
    Metrics,
    Info,
    Tool { name: String, version: String },
    End,
    Version,
}

async fn send_setup_request(socket_path: &str, api_key: String)
{
    let mut socket = UnixStream::connect(socket_path).await.expect("Failed to connect to unix socket");
    let setup_request = json!({
            "command": "setup",
            "api_key": api_key
    });
    let setup_request_json = serde_json::to_string(&setup_request).expect("Failed to serialize setup request");
    socket.write_all(setup_request_json.as_bytes()).await.expect("Failed to connect to the daemon");
}

async fn send_log_request(socket_path: &str, message: String)
{
    let mut socket = UnixStream::connect(socket_path).await.expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "log",
            "message": message
    });
    let start_request_json = serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket.write_all(start_request_json.as_bytes()).await.expect("Failed to connect to the daemon");
}

async fn send_alert_request(socket_path: &str, message: String)
{
    let mut socket = UnixStream::connect(socket_path).await.expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "alert",
            "message": message
    });
    let start_request_json = serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket.write_all(start_request_json.as_bytes()).await.expect("Failed to connect to the daemon");
}

async fn send_init_request(socket_path: &str)
{
    let mut socket = UnixStream::connect(socket_path).await.expect("Failed to connect to unix socket");
    let start_request = json!({
            "command": "init"
    });
    let start_request_json = serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket.write_all(start_request_json.as_bytes()).await.expect("Failed to connect to the daemon");
}

pub async fn parse_input(socket_path: &str) {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { api_key } => {
            send_setup_request(socket_path, api_key).await
        }
        Commands::Log { message } => {
            send_log_request(socket_path, message).await
        }
        Commands::Alert { message } => {
            send_alert_request(socket_path, message).await
        }
        Commands::Init => {
            send_init_request(socket_path).await
        }
        _ => {
            println!("Command not implemented yet");
        }
    };
}
