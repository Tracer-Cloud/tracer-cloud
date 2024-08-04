pub mod presigned_url_put;
pub mod upload_to_signed_url;

use anyhow::{Context, Result};
use presigned_url_put::request_presigned_url;
use std::fs;
use std::path::Path;

use crate::{
    config_manager::ConfigManager, upload::upload_to_signed_url::upload_file_to_signed_url_s3,
};

pub async fn upload_from_file_path(file_path: &str) -> Result<()> {
    const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB in bytes

    // Step #1: Check if the file exists
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("The file '{}' does not exist.", file_path));
    }

    // Step #2: Extract the file name
    let file_name = path
        .file_name()
        .context("Failed to extract file name")?
        .to_str()
        .context("File name is not valid UTF-8")?;

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

    let config = ConfigManager::load_default_config();
    let api_key = config.api_key.clone();

    // Step #4: Request the upload URL
    let signed_url = request_presigned_url(&api_key, file_name).await?;

    // Step #5: Upload the file
    upload_file_to_signed_url_s3(&signed_url, file_path).await?;

    // Log success
    println!("File '{}' has been uploaded successfully.", file_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_upload_from_file_path() -> Result<()> {
        // Use an existing file in your project
        let file_path = "log_outgoing_http_calls.txt";

        // Ensure the file exists before running the test
        assert!(Path::new(file_path).exists(), "Test file does not exist");

        let result = upload_from_file_path(file_path).await;
        assert!(result.is_ok(), "Upload failed: {:?}", result.err());

        Ok(())
    }

    #[tokio::test]
    async fn test_upload_from_file_path_file_not_found() -> Result<()> {
        let file_path = "non_existent_file.txt";

        // Ensure the file does not exist
        assert!(
            !Path::new(file_path).exists(),
            "Test file unexpectedly exists"
        );

        let result = upload_from_file_path(file_path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    }

    #[tokio::test]
    async fn test_upload_from_file_path_file_too_large() -> Result<()> {
        let file_path = "large_test_file.txt";

        // Create a file larger than 5MB
        {
            let mut file = File::create(file_path)?;
            let large_content = vec![0u8; 6 * 1024 * 1024]; // 6MB
            file.write_all(&large_content)?;
        }

        let result = upload_from_file_path(file_path).await;

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
