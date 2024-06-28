use std::{any::Any, borrow::BorrowMut, future::Future, pin::Pin, sync::Arc};

use serde_json::Value;
use tokio::{io::AsyncReadExt, net::UnixListener, sync::Mutex};

use crate::tracer_client::{self, TracerClient};

type ProcessOutput<'a> = Option<Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + 'a + Send>>>;

pub fn process_log_command<'a>(tracer_client: &'a TracerClient, object: &serde_json::Map<String, serde_json::Value>) -> ProcessOutput<'a> {
    if !object.contains_key("message") {
      return None;
    };

  let message = object.get("message").unwrap().as_str().unwrap().to_string();
  Some(Box::pin(tracer_client.http_client.send_log_event(message)))
}

pub fn process_alert_command<'a>(tracer_client: &'a TracerClient, object: &serde_json::Map<String, serde_json::Value>) -> ProcessOutput<'a> {
    if !object.contains_key("message") {
      return None;
    };

  let message = object.get("message").unwrap().as_str().unwrap().to_string();
  Some(Box::pin(tracer_client.http_client.send_alert_event(message)))
}

pub async fn run_server(tracer_client: Arc<Mutex<TracerClient>>, socket_path: &str) {
    let listener = UnixListener::bind(socket_path).expect("Failed to bind to unix socket");
    
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

        let mut message = String::new();

        println!("{:?}", message);
        
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

        {
          let tracer_client = tracer_client.lock().await;
          let result = match command {
            "log" => {
              process_log_command(&tracer_client, object)
            },
            "alert" => {
              process_alert_command(&tracer_client, object)
            },
            _ => {
              eprintln!("Invalid command: {}", command);
              None
            }
          };

          if let Some(future) = result {
            future.await;
          }
        }
    }
}