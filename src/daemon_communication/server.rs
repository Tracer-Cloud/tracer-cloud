use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Ok;
use serde_json::Value;
use tokio::{io::AsyncReadExt, net::UnixListener, sync::Mutex};
use tokio_util::sync::CancellationToken;

use crate::{
    events::{send_alert_event, send_log_event, send_start_run_event},
    tracer_client::TracerClient,
};

type ProcessOutput<'a> =
    Option<Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + 'a + Send>>>;

/*
Example of timelined code, depedant on TracerClient:

pub fn process_log_command<'a>(
    tracer_client: &'a TracerClient,
    object: &serde_json::Map<String, serde_json::Value>,
) -> ProcessOutput<'a> {
    if !object.contains_key("message") {
        return None;
    };

    let message = object.get("message").unwrap().as_str().unwrap().to_string();
    Some(Box::pin(send_log_event(&tracer_client)))
}*/

pub fn process_log_command<'a>(
    service_url: &'a str,
    api_key: &'a str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> ProcessOutput<'a> {
    if !object.contains_key("message") {
        return None;
    };

    let message = object.get("message").unwrap().as_str().unwrap().to_string();
    Some(Box::pin(send_log_event(service_url, api_key, message)))
}

pub fn process_alert_command<'a>(
    service_url: &'a str,
    api_key: &'a str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> ProcessOutput<'a> {
    if !object.contains_key("message") {
        return None;
    };

    let message = object.get("message").unwrap().as_str().unwrap().to_string();
    Some(Box::pin(send_alert_event(service_url, api_key, message)))
}

pub fn process_start_run_command<'a>(service_url: &'a str, api_key: &'a str) -> ProcessOutput<'a> {
    Some(Box::pin(send_start_run_event(service_url, api_key)))
}

pub async fn run_server(
    tracer_client: Arc<Mutex<TracerClient>>,
    socket_path: &str,
    cancellation_token: CancellationToken,
) -> Result<(), anyhow::Error> {
    if std::fs::metadata(socket_path).is_ok() {
        std::fs::remove_file(socket_path).expect("Failed to remove existing socket file");
    }
    let listener = UnixListener::bind(socket_path).expect("Failed to bind to unix socket");

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

        let mut message = String::new();

        let result = stream.read_to_string(&mut message).await;

        if result.is_err() {
            eprintln!("Error reading from socket: {}", result.err().unwrap());
            continue;
        }

        let json_parse_result = serde_json::from_str(&message);

        if json_parse_result.is_err() {
            eprintln!("Error parsing JSON: {}", json_parse_result.err().unwrap());
            continue;
        }

        let parsed: Value = json_parse_result.unwrap();

        if !parsed.is_object() {
            eprintln!("Invalid JSON received: {}", message);
            continue;
        }

        let object = parsed.as_object().unwrap();

        if !object.contains_key("command") {
            eprintln!("Invalid JSON, no command field, received: {}", message);
            continue;
        }

        let command = object.get("command").unwrap().as_str().unwrap();

        let (service_url, api_key) = {
            let tracer_client = tracer_client.lock().await;
            let service_url = tracer_client.get_service_url().to_owned();
            let api_key = tracer_client.get_api_key().to_owned();
            (service_url, api_key)
        };

        let result = match command {
            "log" => process_log_command(&service_url, &api_key, object),
            "alert" => process_alert_command(&service_url, &api_key, object),
            "start" => process_start_run_command(&service_url, &api_key),
            "stop" => {
                cancellation_token.cancel();
                return Ok(());
            }
            _ => {
                eprintln!("Invalid command: {}", command);
                None
            }
        };

        if let Some(future) = result {
            future.await?;
        }
    }
}
