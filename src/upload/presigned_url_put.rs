use anyhow::{Context, Result};
use serde_json::{json, Value};
use url::Url;

use crate::http_client::send_http_body;

pub async fn request_presigned_url(api_key: &str, file_name: &str) -> Result<String> {
    // @todo: this service url needs to be set automatically by the CLI and be develop or prod based on the environment (currentyl the default rust client api key is from production though so better to keep this as production as well)
    let service_url = "https://app.tracer.bio/api/upload/presigned-put".to_string();

    // Construct the full URL with the query parameter
    let mut url = Url::parse(&service_url).context("Failed to parse service URL")?;
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

        Ok(presigned_url)
    } else {
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

    #[tokio::test]
    async fn test_request_presigned_url() -> Result<()> {
        // Load the configuration
        let config = ConfigManager::load_default_config();
        let api_key = config.api_key.clone();

        // Test file name
        let file_name = "log_outgoing_http_calls.txt";

        // Call the function
        let presigned_url = request_presigned_url(&api_key, file_name).await?;

        // Validate the returned presigned URL
        let url = Url::parse(&presigned_url)?;

        // Check if the URL is valid
        assert!(url.scheme() == "https", "URL scheme should be https");
        assert!(url.host_str().is_some(), "URL should have a host");

        // Check if the URL contains the file name
        assert!(
            url.path().contains(file_name),
            "URL should contain the file name"
        );

        // Check if the URL contains required query parameters
        let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(
            query_pairs.iter().any(|(k, _)| k == "X-Amz-Signature"),
            "URL should contain X-Amz-Signature"
        );

        Ok(())
    }
}
