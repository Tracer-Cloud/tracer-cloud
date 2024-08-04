use anyhow::{Ok, Result};
use core::panic;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{Mutex, RwLock},
};
use tokio_util::sync::CancellationToken;

use crate::{
    config_manager::{Config, ConfigManager},
    debug_log::Logger,
    events::{
        send_alert_event, send_end_run_event, send_log_event, send_start_run_event,
        send_update_tags_event,
    },
    process_watcher::ShortLivedProcessLog,
    tracer_client::TracerClient,
    upload::presigned_url_put::request_presigned_url,
};

type ProcessOutput<'a> =
    Option<Pin<Box<dyn Future<Output = Result<String, anyhow::Error>> + 'a + Send>>>;

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

pub fn process_start_run_command<'a>(
    service_url: &'a str,
    api_key: &'a str,
    stream: &'a mut UnixStream,
) -> ProcessOutput<'a> {
    async fn fun<'a>(
        service_url: &'a str,
        api_key: &'a str,
        stream: &'a mut UnixStream,
    ) -> Result<String, anyhow::Error> {
        let out = send_start_run_event(service_url, api_key).await?;

        #[derive(Deserialize)]
        struct RunLogOutProperties {
            run_name: String,
        }

        #[derive(Deserialize)]
        struct RunLogOut {
            properties: RunLogOutProperties,
        }

        #[derive(Deserialize)]
        struct RunLogResult {
            result: Vec<RunLogOut>,
        }

        let value: RunLogResult = serde_json::from_str(&out).unwrap();

        if value.result.len() != 1 {
            return Err(anyhow::anyhow!("Invalid response from server"));
        }

        let run_name = &value.result[0].properties.run_name;

        let output = json!({
            "run_name": run_name
        });

        stream
            .write_all(serde_json::to_string(&output)?.as_bytes())
            .await?;

        stream.flush().await?;

        Ok("".to_string())
    }

    Some(Box::pin(fun(service_url, api_key, stream)))
}

pub fn process_end_run_command<'a>(service_url: &'a str, api_key: &'a str) -> ProcessOutput<'a> {
    Some(Box::pin(send_end_run_event(service_url, api_key)))
}

pub fn process_refresh_config_command<'a>(
    tracer_client: &'a Arc<Mutex<TracerClient>>,
    config: &'a Arc<RwLock<Config>>,
) -> ProcessOutput<'a> {
    let config_file = ConfigManager::load_config();

    async fn fun<'a>(
        tracer_client: &'a Arc<Mutex<TracerClient>>,
        config: &'a Arc<RwLock<Config>>,
        config_file: crate::config_manager::Config,
    ) -> Result<String, anyhow::Error> {
        tracer_client.lock().await.reload_config_file(&config_file);
        config.write().await.clone_from(&config_file);
        Ok("".to_string())
    }

    Some(Box::pin(fun(tracer_client, config, config_file)))
}

pub fn process_tag_command<'a>(
    service_url: &'a str,
    api_key: &'a str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> ProcessOutput<'a> {
    if !object.contains_key("tags") {
        return None;
    };

    let tags_json = object.get("tags").unwrap().as_array().unwrap();

    let tags: Vec<String> = tags_json
        .iter()
        .map(|tag| tag.as_str().unwrap().to_string())
        .collect();

    Some(Box::pin(send_update_tags_event(service_url, api_key, tags)))
}

pub fn process_log_short_lived_process_command<'a>(
    tracer_client: &'a Arc<Mutex<TracerClient>>,
    object: &serde_json::Map<String, serde_json::Value>,
) -> ProcessOutput<'a> {
    if !object.contains_key("log") {
        return None;
    };

    let log: ShortLivedProcessLog =
        serde_json::from_value(object.get("log").unwrap().clone()).unwrap();

    Some(Box::pin(async move {
        let mut tracer_client = tracer_client.lock().await;
        tracer_client.fill_logs_with_short_lived_process(log)?;
        Ok("".to_string())
    }))
}

pub fn process_upload_command<'a>(_service_url: &'a str, api_key: &'a str) -> ProcessOutput<'a> {
    let logger = Logger::new();
    let file_name = "log_outgoing_http_calls.txt";

    Some(Box::pin(async move {
        let _ = logger.log("server.rs//process_upload_command", None).await;

        request_presigned_url(api_key, file_name).await?;

        logger.log("process_upload_command completed", None).await?;
        Ok("Upload command processed".to_string())
    }))
}

pub async fn run_server(
    tracer_client: Arc<Mutex<TracerClient>>,
    socket_path: &str,
    cancellation_token: CancellationToken,
    config: Arc<RwLock<Config>>,
) -> Result<(), anyhow::Error> {
    if std::fs::metadata(socket_path).is_ok() {
        std::fs::remove_file(socket_path)
            .unwrap_or_else(|_| panic!("Failed to remove existing socket file"));
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
            "terminate" => {
                cancellation_token.cancel();
                return Ok(());
            }
            "log" => process_log_command(&service_url, &api_key, object),
            "alert" => process_alert_command(&service_url, &api_key, object),
            "start" => process_start_run_command(&service_url, &api_key, &mut stream),
            "end" => process_end_run_command(&service_url, &api_key),
            "refresh_config" => process_refresh_config_command(&tracer_client, &config),
            "tag" => process_tag_command(&service_url, &api_key, object),
            "log_short_lived_process" => {
                process_log_short_lived_process_command(&tracer_client, object)
            }
            "ping" => None,
            "upload" => process_upload_command(&service_url, &api_key),
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
