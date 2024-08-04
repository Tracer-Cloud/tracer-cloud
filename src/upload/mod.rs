use log::{debug, error, info};
use reqwest::Client;
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub mod presigned_url_put;

#[derive(Debug)]
pub enum UploadError {
    #[allow(dead_code)]
    FileReadError(std::io::Error),
    #[allow(dead_code)]
    RequestError(reqwest::Error),
    #[allow(dead_code)]
    UploadFailed(String),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UploadError::FileReadError(e) => write!(f, "Failed to read file: {}", e),
            UploadError::RequestError(e) => write!(f, "HTTP request failed: {}", e),
            UploadError::UploadFailed(s) => write!(f, "Upload failed: {}", s),
        }
    }
}

impl Error for UploadError {}

#[allow(dead_code)]
pub async fn upload_file_to_s3(signed_url: &str, file_path: &str) -> Result<(), UploadError> {
    info!("Starting file upload to S3");
    debug!("Signed URL: {}", signed_url);
    debug!("File path: {}", file_path);

    // Create a new HTTP client
    let client = Client::new();

    // Open the file
    let mut file = File::open(file_path).map_err(|e| {
        error!("Failed to open file: {}", e);
        UploadError::FileReadError(e)
    })?;

    let mut contents = Vec::new();
    file.read_to_end(&mut contents).map_err(|e| {
        error!("Failed to read file contents: {}", e);
        UploadError::FileReadError(e)
    })?;

    debug!("File size: {} bytes", contents.len());

    // Send the PUT request
    info!("Sending PUT request to S3");
    let response = client
        .put(signed_url)
        .body(contents)
        .header("Content-Type", "application/octet-stream")
        .send()
        .await
        .map_err(|e| {
            error!("Failed to send request: {}", e);
            UploadError::RequestError(e)
        })?;

    // Check if the upload was successful
    let status = response.status();
    debug!("Response status: {}", status);

    if status.is_success() {
        info!("File uploaded successfully!");
        Ok(())
    } else {
        let error_message = format!("Upload failed with status: {}", status);
        error!("{}", error_message);
        Err(UploadError::UploadFailed(error_message))
    }
}

// #[cfg(test)]
// mod tests {
//     use presigned_url_put::request_presigned_url;

//     use crate::config_manager::ConfigManager;

//     use super::*;

//     #[tokio::test]
//     async fn test_upload_file_to_s3_execution() {
//         // Initialize the logger for tests
//         let _ = env_logger::builder().is_test(true).try_init();

//         // Use the existing file path
//         let file_path = "log_outgoing_http_calls.txt";

//         // Check if the file exists
//         if !std::path::Path::new(file_path).exists() {
//             panic!("The test file '{}' does not exist. Please ensure the file is present before running the test.", file_path);
//         }

//         let config = ConfigManager::load_default_config();
//         let api_key = config.api_key.clone();

//         let signed_url = request_presigned_url(&api_key, &file_path).await.unwrap();

//         let result = upload_file_to_s3(&signed_url, file_path).await;

//         // Assert the result
//         assert!(result.is_ok());
//     }
// }
