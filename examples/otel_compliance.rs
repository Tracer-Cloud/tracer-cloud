use reqwest::Client;
use serde_json::json;
use serde_json::Value;
use tracer::config_manager::ConfigManager;
use tracer::exporters::FsExportHandler;
use tracer::exporters::S3ExportHandler;
use tracer::monitor_processes_with_tracer_client;
use tracer::tracer_client::TracerClient;
use tracer::tracing::init_tracing;

use tracer::types::config::AwsConfig;
use tracer::types::event::Event;

use sysinfo::System;

#[tokio::main]
async fn main() {
    let (loki_task, loki_task_controller) = init_tracing().expect("Failed to init tracing");

    tokio::spawn(loki_task);

    let run_name = format!("local_otel_compliance");
    let mut system = System::new();
    let raw_config = ConfigManager::load_config();

    let export_dir =
        ConfigManager::get_tracer_parquet_export_dir().expect("Failed to get export dir");

    let fs_handler = FsExportHandler::new(export_dir, None);
    let s3_handler = S3ExportHandler::new(
        fs_handler,
        //AwsConfig::Profile("me".to_string()),
        raw_config.aws_init_type.clone(),
        raw_config.aws_region.as_str(),
    )
    .await;

    let current_working_directory =
        std::env::current_dir().expect("Failed to get current working dir");

    let mut client = TracerClient::new(
        raw_config.clone(),
        current_working_directory.to_str().unwrap().to_string(),
        s3_handler,
    )
    .await
    .expect("failed to create client");

    let mut count = 5;

    while count > 0 {
        monitor_processes_with_tracer_client(&mut client)
            .await
            .expect("Failed to monitor_processes_with_tracer_client");
        count -= 1;
        std::thread::sleep(std::time::Duration::from_millis(100));
        // system.refresh_all();
    }

    let data = client.logs.get_events();

    for event in data.iter() {
        push_to_loki(event).await.expect("Failed to push to loki")
    }

    loki_task_controller.shutdown().await;
}

// to manually push to loki the json format is as below, now per the logs,
// https://grafana.com/docs/loki/latest/reference/loki-http-api/#:~:text=You%20can%20optionally,structured%20metadata%20attached%3A
// loki accepts structured metadata which is of type hashmap<String, String>, so the work around is
// to dump the events table as json string which works beautifullying on grafana
//
// What does this mean?:
// We are able to use logql to query logs, and grafana already can infer and build panels from logs
// Through loki, grafana now understands our json fields and can generate fields from it all which
// makes queriability easy

async fn push_to_loki(event: &Event) -> Result<(), Box<dyn std::error::Error>> {
    // Loki endpoint
    let loki_url = "http://localhost:3100/loki/api/v1/push";

    // Create an HTTP client
    let client = Client::new();

    // Current timestamp in nanoseconds
    // let timestamp = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)?
    //     .as_nanos()
    //     .to_string();

    // JSON payload for Loki

    let message = format!(
        "{}",
        serde_json::to_string(&event).expect("failed to get messsage")
    );

    let payload = json!({
        "streams": [
            {
                "stream": {
                    "job": "rust-example-2",
                    "host": "localhost-2" },
                "values": [
            (
                &event.timestamp.timestamp_nanos().to_string(),
                message.clone(),
                json!({"message_type": "json"})
            )
                ]
            }
        ]
    });

    // Send POST request to Loki
    let response = client
        .post(loki_url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    // Check the response status
    if response.status().is_success() {
        println!("Log successfully sent to Loki!");
    } else {
        eprintln!("Failed to send log: {:?}", response.text().await?);
    }

    let mut log_message = String::new();

    flatten_and_log(
        &serde_json::to_value(&event).expect("Failed"),
        None,
        &mut log_message,
    );

    tracing::info!(log_message);

    Ok(())
}

fn flatten_and_log_two(
    value: &serde_json::Value,
    prefix: Option<String>,
    log_message: &mut String,
) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, val) in fields {
                let full_key = if let Some(prefix) = &prefix {
                    format!("{}_{}", prefix, key)
                } else {
                    key.clone()
                };
                flatten_and_log_two(val, Some(full_key), log_message);
            }
        }
        _ => {
            let key = prefix.unwrap_or_else(|| "unknown".to_string());
            log_message.push_str(&format!("{}={:?} ", key, value));
        }
    }
}

fn flatten_and_log(value: &Value, prefix: Option<String>, log_message: &mut String) {
    fn sanitize_key(key: &str) -> String {
        key.replace(' ', "_")
    }

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_prefix = match &prefix {
                    Some(pre) => format!("{}_{}", sanitize_key(pre), sanitize_key(key)),
                    None => sanitize_key(key),
                };
                flatten_and_log(val, Some(new_prefix), log_message);
            }
        }
        Value::Array(arr) => {
            for (index, val) in arr.iter().enumerate() {
                let new_prefix = match &prefix {
                    Some(pre) => format!("{}_{}", sanitize_key(pre), index),
                    None => format!("[{}]", index),
                };
                flatten_and_log(val, Some(new_prefix), log_message);
            }
        }
        Value::Bool(b) => {
            let key = prefix.unwrap_or_else(|| "unknown".to_string());
            log_message.push_str(&format!("{}={} ", key, b));
        }
        Value::Number(n) => {
            let key = prefix.unwrap_or_else(|| "unknown".to_string());
            log_message.push_str(&format!("{}={} ", key, n));
        }
        Value::String(s) => {
            let key = prefix.unwrap_or_else(|| "unknown".to_string());
            log_message.push_str(&format!("{}={} ", key, s));
        }
        Value::Null => {
            let key = prefix.unwrap_or_else(|| "unknown".to_string());
            log_message.push_str(&format!("{}=null ", key));
        }
    }
}
