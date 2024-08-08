pub mod presigned_url_put;
pub mod upload_to_signed_url;

use anyhow::{Context, Result};
use presigned_url_put::request_presigned_url;
use std::fs;
use std::path::Path;

use crate::debug_log::Logger;
use crate::upload::upload_to_signed_url::upload_file_to_signed_url_s3;

pub async fn upload_from_file_path(
    service_url: &str,
    api_key: &str,
    file_path: &str,
    custom_file_name: Option<&str>,
) -> Result<()> {
    const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB in bytes

    let logger = Logger::new();

    // Step #1: Check if the file exists
    let path = Path::new(file_path);
    if !path.exists() {
        logger
            .log(&format!("The file '{}' does not exist.", file_path), None)
            .await;
        return Err(anyhow::anyhow!("The file '{}' does not exist.", file_path));
    }

    logger
        .log(&format!("The file '{}' exists.", file_path), None)
        .await;

    // Step #2: Extract the file name
    let file_name = if let Some(file_name) = custom_file_name {
        file_name
    } else {
        path.file_name()
            .context("Failed to extract file name")?
            .to_str()
            .context("File name is not valid UTF-8")?
    };

    logger
        .log(&format!("Uploading file '{}'", file_name), None)
        .await;

    // Step #3: Check if the file is under 5MB
    let metadata = fs::metadata(file_path)?;
    let file_size = metadata.len();
    if file_size > MAX_FILE_SIZE {
        println!(
            "Warning: File size ({} bytes) exceeds 5MB limit.",
            file_size
        );
        return Err(anyhow::anyhow!("File size exceeds 5MB limit"));
    }

    logger
        .log(&format!("File size: {} bytes", file_size), None)
        .await;

    // Step #4: Request the upload URL
    let signed_url = request_presigned_url(service_url, api_key, file_name).await?;

    logger
        .log(&format!("Presigned URL: {}", signed_url), None)
        .await;

    // Step #5: Upload the file
    upload_file_to_signed_url_s3(&signed_url, file_path).await?;

    logger.log("File uploaded successfully", None).await;

    // Log success
    println!("File '{}' has been uploaded successfully.", file_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config_manager::ConfigManager;

    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_upload_from_file_path() -> Result<()> {
        // Use an existing file in your project
        let file_path = "log_outgoing_http_calls.txt";
        let config = ConfigManager::load_default_config();

        // Ensure the file exists before running the test
        assert!(Path::new(file_path).exists(), "Test file does not exist");

        let result =
            upload_from_file_path(&config.service_url, &config.api_key, file_path, None).await;
        assert!(result.is_ok(), "Upload failed: {:?}", result.err());

        Ok(())
    }

    #[tokio::test]
    async fn test_upload_from_file_path_file_not_found() -> Result<()> {
        let file_path = "non_existent_file.txt";
        let config = ConfigManager::load_default_config();

        // Ensure the file does not exist
        assert!(
            !Path::new(file_path).exists(),
            "Test file unexpectedly exists"
        );

        let result =
            upload_from_file_path(&config.service_url, &config.api_key, file_path, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    }

    #[tokio::test]
    async fn test_upload_from_file_path_file_too_large() -> Result<()> {
        let file_path = "large_test_file.txt";
        let config = ConfigManager::load_default_config();

        // Create a file larger than 5MB
        {
            let mut file = File::create(file_path)?;
            let large_content = vec![0u8; 6 * 1024 * 1024]; // 6MB
            file.write_all(&large_content)?;
        }

        let result =
            upload_from_file_path(&config.service_url, &config.api_key, file_path, None).await;
        // Clean up the large file
        fs::remove_file(file_path)?;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds 5MB limit"));

        Ok(())
    }
}
