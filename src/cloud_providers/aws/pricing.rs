use aws_sdk_pricing as pricing;
use aws_sdk_pricing::types::Filter as PricingFilters;

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
        let mut response = self
            .client
            .get_products()
            .service_code("AmazonEC2".to_string())
            .set_filters(Some(filters))
            .into_paginator()
            .send();

        let mut data = Vec::new();

        while let Some(Ok(output)) = response.next().await {
            for product in output.price_list() {
                let pricing: PricingData = serde_json::from_str::<Query<PricingData>>(product)
                    .unwrap()
                    .into();
                let flat_data = FlattenedData::flatten_data(&pricing);
                data.push(flat_data);
            }
        }

        data.into_iter().reduce(|a, b| {
            if a.price_per_unit > b.price_per_unit {
                a
            } else {
                b
            }
        })
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

    // Test multiple instance types
    #[tokio::test]
    async fn test_multiple_instance_types() {
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
        ];

        let result = client.get_ec2_instance_price(filters).await;
        assert!(result.is_some());
        let price_data = result.unwrap();
        assert!(price_data.price_per_unit > 0.0);
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
