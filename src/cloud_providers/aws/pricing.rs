use aws_sdk_pricing as pricing;
use aws_sdk_pricing::types::Filter as PricingFilters;
use log::{debug, error, warn};

use crate::types::{
    aws::pricing::{FlattenedData, PricingData},
    config::AwsConfig,
};
use serde_query::Query;

use super::get_initialized_aws_conf;

pub struct PricingClient {
    pub client: pricing::client::Client,
}

impl PricingClient {
    // for now only us-east-1 worked so i'm sticking to that
    pub async fn new(initialization_conf: AwsConfig, _region: &'static str) -> Self {
        let region = "us-east-1";
        let config = get_initialized_aws_conf(initialization_conf, region).await;

        Self {
            client: pricing::client::Client::new(&config),
        }
    }

    /// For now this method returns the most expensive ec2 instance based on the filters.
    /// This is because for now i haven't figured out the a way to narrow down the results into
    /// one value. But the idea is we can estimate since the price for similar configurations are
    /// very close
    pub async fn get_ec2_instance_price(
        &self,
        filters: Vec<PricingFilters>,
    ) -> Option<FlattenedData> {
        // 1. Create a paginated request to AWS Pricing API
        let mut response = self
            .client
            .get_products()
            .service_code("AmazonEC2".to_string())  // Specifically query EC2 prices
            .set_filters(Some(filters))             // Apply the filters (instance type, OS, etc)
            .into_paginator()                       // Handle pagination of results
            .send();

        let mut data = Vec::new();

        // 2. Process each page of results
        while let Some(Ok(output)) = response.next().await {
            // 3. Process each product in the current page
            for product in output.price_list() {
                // 4. Parse the JSON pricing data using serde_query with error handling
                match serde_json::from_str::<Query<PricingData>>(product) {
                    Ok(pricing) => {
                        // 5. Convert the complex pricing data into a flattened format
                        let flat_data = FlattenedData::flatten_data(&pricing.into());
                        data.push(flat_data);
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse product data: {:?}\nProduct: {}", 
                            e, product
                        );
                        continue;  // Skip this product and continue with the next one
                    }
                }
            }
        }

        debug!("Processed pricing data length: {}", data.len());
        
        // 6. Find the most expensive instance (only if we have data)
        if data.is_empty() {
            warn!("No valid pricing data found");
            None
        } else {
            data.into_iter().reduce(|a, b| {
                if a.price_per_unit > b.price_per_unit {
                    a
                } else {
                    b
                }
            })
        }
    }
}

// e2e S3 tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::config::AwsConfig;
    use aws_sdk_pricing::types::{Filter, FilterType};
    use dotenv::dotenv;
    use std::time::Duration;
    use tokio;
    use tokio::time::timeout;

    async fn setup_client() -> PricingClient {
        dotenv().ok();
        let config = AwsConfig::Env;
        PricingClient::new(config, "us-east-1").await
    }

    // Basic functionality test
    #[tokio::test]
    async fn test_get_ec2_instance_price() {
        let client = setup_client().await;
        let filters = vec![
            Filter::builder()
                .field("instanceType")
                .value("t2.micro")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("operatingSystem")
                .value("Linux")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("tenancy")
                .value("Shared")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("location")
                .value("US East (N. Virginia)")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
        ];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_some());

        let price_data = result.unwrap();
        assert_eq!(price_data.instance_type, "t2.micro");
        assert!(price_data.price_per_unit > 0.0);
        assert_eq!(price_data.unit, "Hrs");
    }

    // Test no results case
    #[tokio::test]
    async fn test_no_matching_instances() {
        let client = setup_client().await;
        let filters = vec![Filter::builder()
            .field("instanceType")
            .value("non_existent_instance_type")
            .r#type(FilterType::TermMatch)
            .build()
            .unwrap()];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_none());
    }

    // Test multiple shared instance types
    #[tokio::test]
    async fn test_multiple_instance_types_with_shared_tenancy() {
        let client = setup_client().await;
        let filters = vec![
            Filter::builder()
                .field("instanceType")
                .value("t2.micro")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("operatingSystem")
                .value("Linux")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("tenancy")
                .value("Shared")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("location")
                .value("US East (N. Virginia)")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
        ];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_some());

        let price_data = result.unwrap();
        assert!(price_data.price_per_unit > 0.0);
    }

    // Test multiple shared and reserved instance types
    #[tokio::test]
    async fn test_multiple_instance_types_with_shared_and_reserved_tenancy() {
        let client = setup_client().await;
        let filters = vec![
            Filter::builder()
                .field("instanceType")
                .value("t2.micro")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("operatingSystem")
                .value("Linux")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("location")
                .value("US East (N. Virginia)")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
        ];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_some());

        let price_data = result.unwrap();
        assert!(price_data.price_per_unit > 0.0);
    }

    // Test multiple reserved instance types
    #[tokio::test]
    async fn test_multiple_instance_types_with_reserved_tenancy() {
        let client = setup_client().await;
        let filters = vec![
            Filter::builder()
                .field("operatingSystem")
                .value("Linux")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("location")
                .value("US East (N. Virginia)")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
            Filter::builder()
                .field("tenancy")
                .value("Reserved")
                .r#type(FilterType::TermMatch)
                .build()
                .unwrap(),
        ];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_none());
    }

    // Test with timeout
    #[tokio::test]
    async fn test_request_timeout() {
        let client = setup_client().await;
        let filters = vec![Filter::builder()
            .field("instanceType")
            .value("t2.micro")
            .r#type(FilterType::TermMatch)
            .build()
            .unwrap()];

        let result = timeout(
            Duration::from_secs(5),
            client.get_ec2_instance_price(filters),
        )
        .await;

        assert!(result.is_ok());
        let price_data = result.unwrap();
        assert!(price_data.is_some());
    }
}
