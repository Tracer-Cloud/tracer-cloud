use anyhow::{Context, Result};
use serde_json::{json, Value};
use url::Url;

use crate::utils::{debug_log::Logger, http_client::send_http_body};

pub async fn request_presigned_url(
    service_url: &str,
    api_key: &str,
    file_name: &str,
) -> Result<String> {
    // Construct the full URL with the query parameter
    let presigned_url = format!("{}/upload/presigned-put", service_url);
    let logger = Logger::new();
    let mut url = Url::parse(&presigned_url).context("Failed to parse service URL")?;
    url.query_pairs_mut().append_pair("fileName", file_name);

    // Prepare the request body (empty in this case)
    let request_body = json!({});

    // Send the request
    let (status, response_text) = send_http_body(url.as_str(), api_key, &request_body).await?;

    if (200..300).contains(&status) {
        // Parse the response to extract the presigned URL
        let response: Value =
            serde_json::from_str(&response_text).context("Failed to parse response JSON")?;

        let presigned_url = response["signedUrl"]
            .as_str()
            .context("Presigned URL not found in response")?
            .to_string();

        logger
            .log(&format!("Presigned URL: {}", presigned_url), None)
            .await;

        Ok(presigned_url)
    } else {
        logger
            .log(
                &format!(
                    "Failed to get presigned URL. Status: {}, Response: {}",
                    status, response_text
                ),
                None,
            )
            .await;

        Err(anyhow::anyhow!(
            "Failed to get presigned URL. Status: {}, Response: {}",
            status,
            response_text,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::ConfigManager;
    use dotenv::dotenv;
    use uuid::Uuid;

    fn setup_env_vars() {
        dotenv().ok(); // Load from .env file in development
    }

    #[ignore = "deprecated"]
    #[tokio::test]
    async fn test_request_presigned_url() -> Result<()> {
        setup_env_vars();

        // Load the configuration
        let config = ConfigManager::load_default_config();
        let api_key = config.api_key.clone();

        // Generate a unique test file name to avoid conflicts
        let file_name = format!("test-upload-{}.txt", Uuid::new_v4());

        // Call the function
        let presigned_url =
            request_presigned_url(&config.service_url, &api_key, &file_name).await?;

        // Validate the returned presigned URL
        let url = Url::parse(&presigned_url)?;

        // Check if the URL is valid
        assert!(url.scheme() == "https", "URL scheme should be https");
        assert!(url.host_str().is_some(), "URL should have a host");

        // Check if the URL contains the file name
        assert!(
            url.path().contains(&file_name),
            "URL should contain the file name"
        );

        // Check if the URL contains required query parameters for AWS
        let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(
            query_pairs.iter().any(|(k, _)| k == "X-Amz-Signature"),
            "URL should contain X-Amz-Signature"
        );
        assert!(
            query_pairs.iter().any(|(k, _)| k == "X-Amz-Algorithm"),
            "URL should contain X-Amz-Algorithm"
        );
        assert!(
            query_pairs.iter().any(|(k, _)| k == "X-Amz-Credential"),
            "URL should contain X-Amz-Credential"
        );

        Ok(())
    }
}
