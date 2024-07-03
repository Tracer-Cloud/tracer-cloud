// src/cli.rs
use anyhow::Result;
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
        process_polling_interval_ms: Option<u64>,
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
    Test,
    Tag {
        tags: Vec<String>,
    },
    Version,
}

pub async fn send_log_request(socket_path: &str, message: String) -> Result<()> {
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

pub async fn send_alert_request(socket_path: &str, message: String) -> Result<()> {
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

pub async fn send_stop_request(socket_path: &str) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let stop_request = json!({
            "command": "stop"
    });

    let stop_request_json =
        serde_json::to_string(&stop_request).expect("Failed to serialize stop request");

    socket.write_all(stop_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_start_run_request(socket_path: &str) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let start_request = json!({
            "command": "start"
    });
    let start_request_json =
        serde_json::to_string(&start_request).expect("Failed to serialize start request");
    socket.write_all(start_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_end_run_request(socket_path: &str) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let end_request = json!({
            "command": "end"
    });

    let end_request_json =
        serde_json::to_string(&end_request).expect("Failed to serialize start request");

    socket.write_all(end_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_ping_request(socket_path: &str) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let ping_request = json!({
            "command": "ping"
    });

    let ping_request_json =
        serde_json::to_string(&ping_request).expect("Failed to serialize ping request");

    socket.write_all(ping_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_refresh_config_request(socket_path: &str) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let setup_request = json!({
            "command": "refresh_config"
    });

    let setup_request_json =
        serde_json::to_string(&setup_request).expect("Failed to serialize setup request");

    socket.write_all(setup_request_json.as_bytes()).await?;

    Ok(())
}

pub async fn send_update_tags_request(socket_path: &str, tags: &Vec<String>) -> Result<()> {
    let mut socket = UnixStream::connect(socket_path).await?;

    let tag_request = json!({
            "command": "tag",
            "tags": tags
    });

    let tag_request_json =
        serde_json::to_string(&tag_request).expect("Failed to serialize tag request");

    socket.write_all(tag_request_json.as_bytes()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SOCKET_PATH;
    use serial_test::serial;
    use tokio::{io::AsyncReadExt, net::UnixListener};

    fn setup_test_unix_listener() -> UnixListener {
        let _ = env_logger::builder().is_test(true).try_init();
        if std::fs::metadata(SOCKET_PATH).is_ok() {
            std::fs::remove_file(SOCKET_PATH).expect("Failed to remove existing socket file");
        }

        UnixListener::bind(SOCKET_PATH).expect("Failed to bind to unix socket")
    }

    async fn check_listener_value(listener: &UnixListener, expected_value: &str) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let received = std::str::from_utf8(&buffer[..n]).unwrap();
        assert_eq!(received, expected_value);
    }

    #[tokio::test]
    #[serial]
    async fn test_send_log_request() -> Result<()> {
        let listener = setup_test_unix_listener();
        let message = "Test Message".to_string();

        send_log_request(SOCKET_PATH, message.clone()).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "log",
                "message": message
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_alert_request() -> Result<()> {
        let listener = setup_test_unix_listener();
        let message = "Test Message".to_string();

        send_alert_request(SOCKET_PATH, message.clone()).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "alert",
                "message": message
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_stop_request() -> Result<()> {
        let listener = setup_test_unix_listener();

        send_stop_request(SOCKET_PATH).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "stop"
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_start_run_request() -> Result<()> {
        let listener = setup_test_unix_listener();

        send_start_run_request(SOCKET_PATH).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "start"
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_end_run_request() -> Result<()> {
        let listener = setup_test_unix_listener();

        send_end_run_request(SOCKET_PATH).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "end"
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_ping_request() -> Result<()> {
        let listener = setup_test_unix_listener();

        send_ping_request(SOCKET_PATH).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "ping"
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_refresh_config_request() -> Result<()> {
        let listener = setup_test_unix_listener();

        send_refresh_config_request(SOCKET_PATH).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "refresh_config"
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_send_update_tags_request() -> Result<()> {
        let listener = setup_test_unix_listener();
        let tags = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];

        send_update_tags_request(SOCKET_PATH, &tags).await?;

        check_listener_value(
            &listener,
            json!({
                "command": "tag",
                "tags": tags
            })
            .to_string()
            .as_str(),
        )
        .await;

        Ok(())
    }
}
