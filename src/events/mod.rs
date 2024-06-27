// /// src/events/mod.rs
// use crate::{config_manager::ConfigManager, http_client::HttpClient};
// use anyhow::{Context, Result};
// use serde_json::json;

// #[derive(Debug)]
// pub enum EventStatus {
//     NewRun,
// }

// impl EventStatus {
//     pub fn as_str(&self) -> &'static str {
//         match self {
//             EventStatus::NewRun => "new_run",
//         }
//     }
// }

// // pub async fn event_pipeline_run_start_new() -> Result<()> {
// //     let http_client = initialize_http_client().await?;
// //     println!("Starting new pipeline...");

// //     log_event(
// //         &http_client,
// //         EventStatus::NewRun,
// //         "[CLI] Starting pipeline run",
// //     )
// //     .await?;
// //     println!("Started pipeline run successfully...");

// //     Ok(())
// // }

// // async fn log_event(http_client: &HttpClient, status: EventStatus, message: &str) -> Result<()> {
// //     let log_entry = json!({
// //         "message": message,
// //         "process_type": "pipeline",
// //         "process_status": status.as_str(),
// //         "event_type": "process_status"
// //     });

// //     http_client.send_http_event(&log_entry).await
// // }

// // async fn initialize_http_client() -> Result<HttpClient> {
// //     let config = ConfigManager::load_config().context("Failed to load config")?;
// //     let service_url = config.service_url;
// //     let api_key = config.api_key;
// //     let http_client = HttpClient::new(service_url, api_key);
// //     Ok(http_client)
// // }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use anyhow::Error;

//     #[tokio::test]
//     async fn test_event_pipeline_run_start_new() -> Result<(), Error> {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let result = event_pipeline_run_start_new().await;

//         assert!(result.is_ok(), "Expected success, but got an error");
//         Ok(())
//     }

//     #[tokio::test]
//     async fn test_log_event() -> Result<(), Error> {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let config = ConfigManager::load_config().context("Failed to load config")?;
//         let service_url: String = config.service_url.clone(); // Cloning here to avoid moving
//         let api_key = config.api_key.clone(); // Cloning here to avoid moving
//         let http_client = HttpClient::new(service_url, api_key.clone()); // Cloning again to avoid move

//         let message = format!(
//             "[shipping] Test log message from the test suite {}",
//             api_key
//         );

//         let result = log_event(&http_client, EventStatus::NewRun, &message).await;

//         assert!(result.is_ok(), "Expected success, but got an error");
//         Ok(())
//     }

//     #[tokio::test]
//     async fn test_initialize_http_client() -> Result<(), Error> {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let result = initialize_http_client().await;

//         assert!(result.is_ok(), "Expected success, but got an error");
//         Ok(())
//     }
// }
